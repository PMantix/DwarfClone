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
}

/// Infection level at which a wound has festered.
pub const INFECTED: i32 = 300;

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

    pub fn wound(&mut self, part: Part, dmg: i32, edged: bool) -> Wound {
        let i = part.idx();
        self.hp[i] -= dmg;
        self.bleeding += if edged { dmg / 3 + 1 } else { dmg / 6 };
        if edged && dmg >= 3 {
            self.dirty = true;
        }
        let hp = self.hp[i];
        let max = self.max[i];
        if part == Part::Head && hp <= 0 || part == Part::UpperBody && hp <= -max / 2 {
            self.dead = true;
            return Wound::Fatal;
        }
        if part.is_limb() && edged && hp <= -max / 3 {
            self.lost[i] = true;
            self.bleeding += 8;
            return Wound::Sever;
        }
        if hp <= 0 {
            Wound::Mangle
        } else if dmg * 3 >= max {
            Wound::Gash
        } else {
            Wound::Bruise
        }
    }

    /// Advance bleeding and recovery; call every 10 ticks.
    pub fn tick(&mut self) {
        if self.dead {
            return;
        }
        if self.bleeding > 0 {
            self.blood -= self.bleeding;
            self.bleeding -= 1;
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

    pub fn infected(&self) -> bool {
        self.infection >= INFECTED
    }

    pub fn heal(&mut self, amount: i32) {
        if self.bleeding > 0 || self.infected() {
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
