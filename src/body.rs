use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Part {
    Head,
    UpperBody,
    LowerBody,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

pub const PARTS: [Part; 7] = [
    Part::Head,
    Part::UpperBody,
    Part::LowerBody,
    Part::LeftArm,
    Part::RightArm,
    Part::LeftLeg,
    Part::RightLeg,
];

impl Part {
    pub fn idx(self) -> usize {
        PARTS.iter().position(|&p| p == self).unwrap()
    }

    pub fn name(self) -> &'static str {
        match self {
            Part::Head => "head",
            Part::UpperBody => "upper body",
            Part::LowerBody => "lower body",
            Part::LeftArm => "left arm",
            Part::RightArm => "right arm",
            Part::LeftLeg => "left leg",
            Part::RightLeg => "right leg",
        }
    }

    /// Relative chance of being struck.
    fn weight(self) -> i32 {
        match self {
            Part::Head => 10,
            Part::UpperBody => 30,
            Part::LowerBody => 20,
            _ => 10,
        }
    }

    fn base_hp(self) -> f32 {
        match self {
            Part::Head => 20.0,
            Part::UpperBody => 40.0,
            Part::LowerBody => 30.0,
            Part::LeftArm | Part::RightArm => 20.0,
            Part::LeftLeg | Part::RightLeg => 22.0,
        }
    }

    pub fn is_limb(self) -> bool {
        !matches!(self, Part::Head | Part::UpperBody | Part::LowerBody)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wound {
    Bruise,
    Gash,
    Mangle,
    Sever,
    Fatal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Body {
    pub hp: [i32; 7],
    pub max: [i32; 7],
    pub lost: [bool; 7],
    pub blood: i32,
    pub max_blood: i32,
    pub bleeding: i32,
    pub dead: bool,
    /// An open wound that hasn't been cleaned; it may fester.
    #[serde(default)]
    pub dirty: bool,
    /// Grows while a wound is dirty; from INFECTED on it stops healing and does harm.
    #[serde(default)]
    pub infection: i32,
    /// Limbs set in a splint.
    #[serde(default)]
    pub splint: [bool; 7],
    /// Walks with a crutch (for a lost leg).
    #[serde(default)]
    pub crutch: bool,
    /// How fast each part bleeds, in tenths of a unit of blood per body
    /// tick (10 game ticks). `bleeding` is their sum in whole units.
    #[serde(default)]
    pub bleeds: [i32; 7],
    /// A hand held hard on the wound: it clots twice as fast.
    #[serde(default)]
    pub pressed: [bool; 7],
    /// Bound in the field: it clots three times as fast, and a stump clots at all.
    #[serde(default)]
    pub bandaged: [bool; 7],
    /// Tenths of blood lost but not yet taken from `blood`.
    #[serde(default)]
    pub blood_frac: i32,
    /// Hundredths of each part's rate clotted but not yet taken from `bleeds`.
    #[serde(default)]
    pub clot_frac: [i32; 7],
}

/// Infection level at which a wound has festered.
pub const INFECTED: i32 = 300;

/// How wounds bleed and clot. Bleeds are tenths of a unit of blood per body
/// tick (a body has 100 units at full size); clotting is the share of a
/// part's rate, in percent, that stops each body tick. The game reads these
/// from knobs (`Game::bleed_params`); tests and the arena use the defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BleedParams {
    /// An edged blow that only bruises: a cut.
    pub cut: i32,
    /// An edged blow that tears the flesh.
    pub gash: i32,
    /// A crushed or broken part, edged or blunt (it bleeds inside).
    pub mangle: i32,
    /// A severed limb: it doesn't clot until bandaged.
    pub sever: i32,
    /// Wounds to the upper or lower body bleed this much more, in percent.
    pub vital_pct: i32,
    /// Clotting while lying or standing still.
    pub clot_still: i32,
    /// Clotting while walking or fighting.
    pub clot_moving: i32,
}

impl Default for BleedParams {
    fn default() -> Self {
        BleedParams { cut: 3, gash: 6, mangle: 12, sever: 45, vital_pct: 150, clot_still: 6, clot_moving: 2 }
    }
}

impl Default for Body {
    fn default() -> Self {
        Body::new(1.0)
    }
}

impl Body {
    pub fn new(scale: f32) -> Body {
        let mut max = [0; 7];
        for p in PARTS {
            max[p.idx()] = (p.base_hp() * scale).round().max(4.0) as i32;
        }
        let blood = (100.0 * scale) as i32;
        Body {
            hp: max,
            max,
            lost: [false; 7],
            blood,
            max_blood: blood,
            bleeding: 0,
            dead: false,
            dirty: false,
            infection: 0,
            splint: [false; 7],
            crutch: false,
            bleeds: [0; 7],
            pressed: [false; 7],
            bandaged: [false; 7],
            blood_frac: 0,
            clot_frac: [0; 7],
        }
    }

    pub fn pick_part(&self, rng: &mut Rng) -> Part {
        let live: Vec<Part> = PARTS.iter().copied().filter(|p| !self.lost[p.idx()]).collect();
        let total: i32 = live.iter().map(|p| p.weight()).sum();
        let mut r = rng.range(0, total);
        for p in &live {
            r -= p.weight();
            if r < 0 {
                return *p;
            }
        }
        Part::UpperBody
    }

    /// A blow, bleeding by the default amounts.
    pub fn wound(&mut self, part: Part, dmg: i32, edged: bool) -> Wound {
        self.wound_with(part, dmg, edged, &BleedParams::default())
    }

    /// A blow: it hurts the part and opens a bleed on it by what it did.
    pub fn wound_with(&mut self, part: Part, dmg: i32, edged: bool, p: &BleedParams) -> Wound {
        let i = part.idx();
        self.hp[i] -= dmg;
        if edged && dmg >= 3 {
            self.dirty = true;
        }
        let hp = self.hp[i];
        let max = self.max[i];
        let w = if part == Part::Head && hp <= 0 || part == Part::UpperBody && hp <= -max / 2 {
            self.dead = true;
            Wound::Fatal
        } else if part.is_limb() && edged && hp <= -max / 3 {
            self.lost[i] = true;
            Wound::Sever
        } else if hp <= 0 {
            Wound::Mangle
        } else if dmg * 3 >= max {
            Wound::Gash
        } else {
            Wound::Bruise
        };
        let add = match w {
            Wound::Bruise if edged => p.cut,
            Wound::Gash if edged => p.gash,
            Wound::Mangle => p.mangle,
            Wound::Sever => p.sever,
            _ => 0,
        };
        let add = if matches!(part, Part::UpperBody | Part::LowerBody) { add * p.vital_pct / 100 } else { add };
        if add > 0 {
            // A fresh wound under a bandage opens it again.
            self.bleeds[i] += add;
            self.bandaged[i] = false;
            self.sync_bleeding();
        }
        w
    }

    /// A ruined limb taken off altogether (a heavy blow on a mangled
    /// limb): the stump bleeds as a severing does.
    pub fn sever_with(&mut self, part: Part, p: &BleedParams) {
        let i = part.idx();
        if self.lost[i] {
            return;
        }
        self.lost[i] = true;
        self.bleeds[i] += p.sever;
        self.bandaged[i] = false;
        self.sync_bleeding();
    }

    /// Advance bleeding and recovery, lying still; call every 10 ticks.
    #[cfg(test)]
    pub fn tick(&mut self) {
        self.tick_with(&BleedParams::default(), false);
    }

    /// Advance bleeding and recovery; call every 10 ticks. Wounds clot
    /// slower while `moving` (walking or fighting).
    pub fn tick_with(&mut self, p: &BleedParams, moving: bool) {
        if self.dead {
            return;
        }
        // A save from before per-part bleeding: its bleeding goes on the upper body.
        if self.bleeding > 0 && self.bleeds == [0; 7] {
            self.bleeds[Part::UpperBody.idx()] = self.bleeding * 10;
        }
        if self.bleed_rate() > 0 {
            self.bleed_step(p, moving);
            if self.blood <= 0 {
                self.dead = true;
            }
        } else if self.blood < self.max_blood {
            self.blood += 1;
        }
        // Dirty wounds fester; a body that lives long enough fights it off.
        if self.dirty {
            self.infection += 1;
            if self.infection >= 3 * INFECTED {
                self.dirty = false;
            }
        } else if self.infection > 0 {
            self.infection = (self.infection - 2).max(0);
        }
        if self.infected() && self.infection % 50 == 0 {
            if let Some(i) = (0..7).filter(|&i| !self.lost[i]).min_by_key(|&i| self.hp[i] * 100 / self.max[i].max(1)) {
                self.hp[i] -= 1;
            }
        }
    }

    /// One body tick of blood lost and wounds clotting.
    fn bleed_step(&mut self, p: &BleedParams, moving: bool) {
        self.blood_frac += self.bleed_rate();
        self.blood -= self.blood_frac / 10;
        self.blood_frac %= 10;
        let base = if moving { p.clot_moving } else { p.clot_still };
        for i in 0..7 {
            if self.bleeds[i] == 0 || self.lost[i] && !self.bandaged[i] {
                continue;
            }
            let clot = base * if self.bandaged[i] { 3 } else { 1 } * if self.pressed[i] { 2 } else { 1 };
            let c = self.bleeds[i] * clot + self.clot_frac[i];
            // Still, even a small wound closes by at least a tenth each tick.
            let n = if moving { c / 100 } else { (c / 100).max(1) };
            self.clot_frac[i] = c % 100;
            self.bleeds[i] = (self.bleeds[i] - n).max(0);
            if self.bleeds[i] == 0 {
                self.clot_frac[i] = 0;
                self.pressed[i] = false;
            }
        }
        self.sync_bleeding();
    }

    /// `bleeding` is the sum of the parts' bleeds, in whole units rounded up.
    fn sync_bleeding(&mut self) {
        self.bleeding = (self.bleeds.iter().sum::<i32>() + 9) / 10;
    }

    /// Tenths of blood lost per body tick (10 game ticks), all parts together.
    pub fn bleed_rate(&self) -> i32 {
        self.bleeds.iter().sum()
    }

    /// The part bleeding fastest, if any.
    pub fn worst_bleed(&self) -> Option<Part> {
        PARTS.iter().copied().filter(|p| self.bleeds[p.idx()] > 0).max_by_key(|p| (self.bleeds[p.idx()], -(p.idx() as i32)))
    }

    /// Game ticks until this body bleeds to death if it lies still and
    /// nobody helps; None if it isn't bleeding or will clot first.
    pub fn ticks_to_death(&self) -> Option<u64> {
        if self.dead || self.bleed_rate() == 0 {
            return None;
        }
        let p = BleedParams::default();
        let mut b = self.clone();
        for t in 1..=2000u64 {
            b.bleed_step(&p, false);
            if b.blood <= 0 {
                return Some(t * 10);
            }
            if b.bleed_rate() == 0 {
                return None;
            }
        }
        None
    }

    /// A hand pressed on the wound in `part`.
    pub fn press(&mut self, part: Part) {
        self.pressed[part.idx()] = true;
    }

    /// The wound in `part` bound up: it bleeds half as fast and clots sooner.
    pub fn bandage(&mut self, part: Part) {
        let i = part.idx();
        self.bandaged[i] = true;
        self.bleeds[i] /= 2;
        self.sync_bleeding();
    }

    /// Every wound sewn shut: the bleeding stops.
    pub fn suture(&mut self) {
        self.bleeds = [0; 7];
        self.clot_frac = [0; 7];
        self.pressed = [false; 7];
        self.bandaged = [false; 7];
        self.sync_bleeding();
    }

    pub fn infected(&self) -> bool {
        self.infection >= INFECTED
    }

    pub fn heal(&mut self, amount: i32) {
        if self.bleed_rate() > 0 || self.bleeding > 0 || self.infected() {
            return;
        }
        for i in 0..7 {
            if !self.lost[i] {
                // A set bone mends twice as fast.
                let a = if self.splint[i] { amount * 2 } else { amount };
                self.hp[i] = (self.hp[i] + a).min(self.max[i]);
            }
        }
    }

    /// Hurt enough that an animal (or a goblin, eventually) loses its nerve.
    pub fn badly_hurt(&self) -> bool {
        self.blood * 10 < self.max_blood * 6
            || PARTS.iter().any(|p| self.lost[p.idx()] || self.hp[p.idx()] <= 0)
    }

    pub fn unconscious(&self) -> bool {
        !self.dead && (self.blood < self.max_blood / 3 || self.hp[Part::Head.idx()] < self.max[Part::Head.idx()] / 4)
    }

    /// Extra ticks per step from one leg: splints and crutches help.
    fn leg_drag(&self, p: Part) -> u32 {
        let i = p.idx();
        if self.lost[i] {
            if self.crutch { 2 } else { 5 }
        } else if self.hp[i] <= 0 {
            if self.splint[i] { 1 } else { 3 }
        } else {
            0
        }
    }

    /// Extra ticks per step from leg injuries.
    pub fn limp(&self) -> u32 {
        let (l, r) = (self.leg_drag(Part::LeftLeg), self.leg_drag(Part::RightLeg));
        // Both legs gone or broken is much worse than either alone.
        if l > 0 && r > 0 {
            l + r + 4
        } else {
            l + r
        }
    }

    pub fn can_hold(&self) -> bool {
        !self.lost[Part::RightArm.idx()] && self.hp[Part::RightArm.idx()] > 0
    }

    pub fn needs_rest(&self) -> bool {
        self.bleeding > 0
            || self.infected()
            || PARTS.iter().any(|p| !self.lost[p.idx()] && self.hp[p.idx()] * 10 < self.max[p.idx()] * 6)
    }

    pub fn healed(&self) -> bool {
        self.bleeding == 0
            && PARTS.iter().all(|p| self.lost[p.idx()] || self.hp[p.idx()] * 10 >= self.max[p.idx()] * 9)
    }

    pub fn part_status(&self, p: Part) -> &'static str {
        let i = p.idx();
        if self.lost[i] {
            "missing"
        } else if self.hp[i] <= 0 {
            "mangled"
        } else if self.hp[i] * 2 < self.max[i] {
            "badly wounded"
        } else if self.hp[i] < self.max[i] {
            "bruised"
        } else {
            "fine"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Body ticks until `b` dies (in game ticks), or None within `limit` game ticks.
    fn bleed_out(b: &mut Body, moving: bool, limit: u64) -> Option<u64> {
        let p = BleedParams::default();
        for t in 1..=limit / 10 {
            b.tick_with(&p, moving);
            if b.dead {
                return Some(t * 10);
            }
        }
        None
    }

    #[test]
    fn one_gash_lying_still_clots() {
        let mut b = Body::new(1.0);
        assert_eq!(b.wound(Part::LeftLeg, 9, true), Wound::Gash);
        assert!(b.bleed_rate() > 0 && b.bleeding > 0);
        assert_eq!(b.worst_bleed(), Some(Part::LeftLeg));
        assert_eq!(b.ticks_to_death(), None, "one gash shouldn't kill a dwarf lying still");
        assert_eq!(bleed_out(&mut b, false, 3000), None);
        assert!(b.blood > 80, "lost too much blood to one gash: {}", b.blood);
        assert_eq!((b.bleed_rate(), b.bleeding), (0, 0));
    }

    #[test]
    fn a_severed_arm_kills_in_minutes() {
        let mut b = Body::new(1.0);
        assert_eq!(b.wound(Part::LeftArm, 27, true), Wound::Sever);
        let expected = b.ticks_to_death().expect("a stump with nobody to bind it is fatal");
        let died = bleed_out(&mut b, false, 3000).expect("a stump that clots by itself");
        assert!((150..=350).contains(&died), "a severed arm killed in {} ticks", died);
        assert_eq!(died, expected);
        // Bound up, the same wound clots.
        let mut b = Body::new(1.0);
        b.wound(Part::LeftArm, 27, true);
        b.bandage(Part::LeftArm);
        assert_eq!(b.ticks_to_death(), None);
        assert_eq!(bleed_out(&mut b, false, 3000), None);
    }

    fn five_gashes() -> Body {
        let mut b = Body::new(1.0);
        for (part, dmg) in [(Part::LeftArm, 7), (Part::RightArm, 7), (Part::LeftLeg, 8), (Part::RightLeg, 8), (Part::LowerBody, 10)] {
            assert_eq!(b.wound(part, dmg, true), Wound::Gash, "{:?}", part);
        }
        b
    }

    #[test]
    fn five_gashes_fighting_kill_in_hours() {
        let mut b = five_gashes();
        let died = bleed_out(&mut b, true, 5000).expect("five gashes fighting on should kill");
        assert!((400..=900).contains(&died), "five gashes killed in {} ticks", died);
        // Bandaged at tick 100, the same dwarf lives.
        let mut b = five_gashes();
        assert_eq!(bleed_out(&mut b, true, 100), None);
        for p in PARTS {
            if b.bleeds[p.idx()] > 0 {
                b.bandage(p);
            }
        }
        assert_eq!(bleed_out(&mut b, true, 5000), None, "bandaged in time, yet bled out");
        assert_eq!(b.bleed_rate(), 0);
    }

    #[test]
    fn pressing_and_lying_still_save_blood() {
        let lost = |press: bool, moving: bool| {
            let mut b = Body::new(1.0);
            assert_eq!(b.wound(Part::UpperBody, 40, false), Wound::Mangle);
            if press {
                b.press(Part::UpperBody);
            }
            while b.bleed_rate() > 0 && !b.dead {
                b.tick_with(&BleedParams::default(), moving);
            }
            100 - b.blood
        };
        assert!(lost(false, false) < lost(false, true), "moving should bleed more");
        assert!(lost(true, false) < lost(false, false), "pressing should bleed less");
        let mut b = five_gashes();
        b.suture();
        assert_eq!((b.bleed_rate(), b.bleeding, b.worst_bleed()), (0, 0, None));
    }

    #[test]
    fn old_saves_bleed_on_the_upper_body() {
        let old = r#"{"hp":[20,40,30,20,20,22,22],"max":[20,40,30,20,20,22,22],"lost":[false,false,false,false,false,false,false],
            "blood":100,"max_blood":100,"bleeding":5,"dead":false}"#;
        let mut b: Body = serde_json::from_str(old).unwrap();
        assert_eq!(b.bleeds, [0; 7]);
        b.tick();
        assert!(b.bleeds[Part::UpperBody.idx()] > 0 && b.bleed_rate() == b.bleeds[Part::UpperBody.idx()]);
        assert_eq!(b.blood, 95, "an old save's bleeding should flow at its old rate");
        assert_eq!(b.bleeding, (b.bleed_rate() + 9) / 10);
        assert_eq!(b.worst_bleed(), Some(Part::UpperBody));
    }
}
