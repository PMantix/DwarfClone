//! Noises worth hearing that can't be seen from outside the simulation:
//! each blow struck in a fight, and what it did (a blade going in, a bone
//! breaking, a cry at a limb lost, teeth knocked out, retching, a death).
//! The game only notes them (a few at most, dropped when nobody listens);
//! the app plays them (`src/audio`). Noting them reads nothing the fight
//! uses and rolls no dice, so a fight goes the same with sound or without.

use super::combat::Blow;
use super::*;
use crate::body::Hurt;
use crate::creature::Species;

/// Most noises kept between frames.
pub const MAX_SOUNDS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// A blow that landed.
    Blow,
    /// A blow blocked or turned by armour.
    Parry,
    /// A blade (or teeth) deep into flesh.
    Wet,
    /// A bone broken (`true`: the skull).
    Crunch(bool),
    /// A cry at a limb cut off, on a pitch (0 the deepest .. 255 the shrillest).
    Cry(u8),
    /// Someone knocked sprawling.
    Fall,
    /// Teeth knocked out, skittering away.
    Teeth,
    /// A death: the last breath, and the body falling.
    Death,
    Retch,
}

/// How high `who` (None a dwarf) cries out, from the size of their
/// throat, varied a little by `h`: a dragon bellows, a goblin shrieks.
pub fn cry_pitch(who: Option<Species>, h: u64) -> u8 {
    let base: i32 = match who {
        None | Some(Species::Raider) => 120,
        Some(Species::Goblin | Species::Skulker) => 175,
        Some(Species::Elf) => 160,
        Some(Species::Human | Species::Merchant | Species::Visitor) => 135,
        Some(Species::Megabeast | Species::CaveHorror) => 15,
        Some(Species::Undead | Species::Husk) => 95,
        Some(s) => {
            // Beasts by their size: bigger, deeper.
            let scale = s.info().scale.max(0.1);
            (150.0 - 40.0 * scale.log2()).clamp(30.0, 220.0) as i32
        }
    };
    (base + (h % 25) as i32 - 12).clamp(0, 255) as u8
}

impl Game {
    /// Note a blow at `at`: `landed`, or met (`met`) by shield or armour.
    pub(super) fn blow_sound(&mut self, landed: bool, met: bool, at: Pos) {
        let s = if landed {
            Sound::Blow
        } else if met {
            Sound::Parry
        } else {
            return;
        };
        self.note_sound(at, s);
    }

    fn note_sound(&mut self, at: Pos, s: Sound) {
        if self.sounds.len() < MAX_SOUNDS {
            self.sounds.push((at, s));
        }
    }

    /// Note what a blow at `at` did to `who` (None a dwarf): the strike
    /// itself and its gore, the most telling first.
    pub(super) fn blow_sounds(&mut self, b: &Blow, at: Pos, who: Option<Species>) {
        let landed = b.blood > 0 || b.notable;
        let deep = b.severed.is_some() || b.blood >= 3 || b.hurts.iter().any(|h| matches!(h, Hurt::Guts | Hurt::Artery));
        if b.fatal {
            self.note_sound(at, Sound::Death);
        } else if b.severed.is_some() {
            // (A head off is a death, above: no cry.)
            self.note_sound(at, Sound::Cry(cry_pitch(who, super::gore::mix(at, self.tick))));
        }
        if b.hurts.iter().any(|h| matches!(h, Hurt::Skull)) {
            self.note_sound(at, Sound::Crunch(true));
        } else if b.hurts.iter().any(|h| matches!(h, Hurt::Bone | Hurt::Ribs)) {
            self.note_sound(at, Sound::Crunch(false));
        }
        if deep {
            self.note_sound(at, Sound::Wet);
        } else {
            self.blow_sound(landed, b.msg.is_some(), at);
        }
        if b.teeth > 0 {
            self.note_sound(at, Sound::Teeth);
        }
        if b.hurts.iter().any(|h| matches!(h, Hurt::Vomit)) {
            self.note_sound(at, Sound::Retch);
        }
        if b.sprawl && !b.fatal {
            self.note_sound(at, Sound::Fall);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dragon_bellows_and_a_goblin_shrieks() {
        let dwarf = cry_pitch(None, 12);
        assert!(cry_pitch(Some(Species::Megabeast), 12) < 60);
        assert!(cry_pitch(Some(Species::Goblin), 12) > dwarf);
        assert!(cry_pitch(Some(Species::Bear), 12) < cry_pitch(Some(Species::Cat), 12));
        // Two goblins needn't sound alike.
        assert_ne!(cry_pitch(Some(Species::Goblin), 3), cry_pitch(Some(Species::Goblin), 17));
    }

    /// A telling blow is heard for what it did; the noting is capped.
    #[test]
    fn a_blow_is_heard_for_its_gore() {
        let mut g = Game::new(3);
        let at = g.embark;
        let b = Blow { blood: 4, severed: Some(crate::body::Part::LeftArm), teeth: 2, hurts: vec![Hurt::Bone, Hurt::Vomit], sprawl: true, notable: true, ..Blow::default() };
        g.blow_sounds(&b, at, Some(Species::Goblin));
        let heard: Vec<Sound> = g.sounds.iter().map(|s| s.1).collect();
        assert!(heard.iter().any(|s| matches!(s, Sound::Cry(_))), "{:?}", heard);
        for s in [Sound::Crunch(false), Sound::Wet, Sound::Teeth, Sound::Retch, Sound::Fall] {
            assert!(heard.contains(&s), "{:?} missing from {:?}", s, heard);
        }
        g.sounds.clear();
        let dead = Blow { fatal: true, notable: true, blood: 2, hurts: vec![Hurt::Skull], ..Blow::default() };
        g.blow_sounds(&dead, at, None);
        let heard: Vec<Sound> = g.sounds.iter().map(|s| s.1).collect();
        assert!(heard.contains(&Sound::Death) && heard.contains(&Sound::Crunch(true)) && !heard.iter().any(|s| matches!(s, Sound::Cry(_))), "{:?}", heard);
        for _ in 0..100 {
            g.blow_sounds(&b, at, None);
        }
        assert_eq!(g.sounds.len(), MAX_SOUNDS);
    }
}
