//! The clock: hours, night and the moon, from the tick alone (pure).
//!
//! A day is `DAY` ticks and starts at midnight, so tick 0 is the dead of
//! night; the adventure starts its clock at dawn (`DAWN_TICK`). Nothing in
//! the fortress reads this yet: it is the clock PLAN.md's foundations table
//! leaves to whichever of adventurer mode, M6 B13 (surface night) or M7
//! FC1 (the thing in the dark) comes first, and the others extend it.

use super::{DAY, MONTH};

/// Ticks in an hour (a day is 24 of them).
pub const HOUR: u64 = DAY / 24;
/// The tick of six in the morning on the first day.
pub const DAWN_TICK: u64 = HOUR * 6;
/// Night runs from this hour...
pub const DUSK_HOUR: u32 = 20;
/// ...to this one.
pub const DAWN_HOUR: u32 = 6;

/// The hour of the day, 0-23.
pub fn hour(tick: u64) -> u32 {
    ((tick % DAY) / HOUR) as u32
}

/// From eight at night to six in the morning.
#[allow(dead_code)] // G03 A3 (travel) and A2 (sight) read it
pub fn is_night(tick: u64) -> bool {
    let h = hour(tick);
    h >= DUSK_HOUR || h < DAWN_HOUR
}

/// How much light there is (0 dark to 255 full day). Under rock it is always
/// dark; under the sky it ramps through dawn and dusk, and the moon lends a
/// little at night.
pub fn light(tick: u64, under_sky: bool) -> u8 {
    if !under_sky {
        return 0;
    }
    // Minutes into the day, for smooth ramps.
    let m = ((tick % DAY) * 24 * 60 / DAY) as i64;
    let (dawn, dusk) = (DAWN_HOUR as i64 * 60, DUSK_HOUR as i64 * 60);
    let ramp = 90;
    let night = 20 + moon_light(tick) as i64;
    let v = if m < dawn - ramp || m >= dusk + ramp {
        night
    } else if m < dawn + ramp {
        night + (255 - night) * (m - (dawn - ramp)) / (2 * ramp)
    } else if m < dusk - ramp {
        255
    } else {
        255 - (255 - night) * (m - (dusk - ramp)) / (2 * ramp)
    };
    v.clamp(0, 255) as u8
}

/// The moon's phase, 0 (new) to 7; 4 is full. A month is one turn of it.
pub fn moon(tick: u64) -> u8 {
    ((tick % MONTH) * 8 / MONTH) as u8
}

/// The moon's light at night: nothing at new moon, most at full.
fn moon_light(tick: u64) -> u8 {
    let p = moon(tick) as i32;
    (40 - (p - 4).abs() * 10) as u8
}

/// The time of day in words.
pub fn time_words(tick: u64) -> &'static str {
    match hour(tick) {
        0..=2 => "the dead of night",
        3..=4 => "before dawn",
        5..=6 => "dawn",
        7..=10 => "morning",
        11..=13 => "midday",
        14..=17 => "afternoon",
        18..=19 => "dusk",
        20..=22 => "night",
        _ => "midnight",
    }
}

/// The moon in words.
pub fn moon_words(tick: u64) -> &'static str {
    ["a new moon", "a waxing crescent", "a half moon, waxing", "a waxing moon", "a full moon", "a waning moon", "a half moon, waning", "a waning crescent"][moon(tick) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_falls_and_the_moon_turns() {
        assert_eq!(hour(0), 0);
        assert_eq!(hour(DAWN_TICK), 6);
        assert_eq!(hour(DAY - 1), 23);
        assert!(is_night(0) && is_night(HOUR * 5) && !is_night(DAWN_TICK) && !is_night(HOUR * 12) && is_night(HOUR * 21));
        // Midday is full light, midnight is dark, and underground is darker still.
        assert_eq!(light(HOUR * 12, true), 255);
        assert!(light(0, true) < 80);
        assert_eq!(light(HOUR * 12, false), 0);
        // Dawn climbs and dusk falls.
        assert!(light(HOUR * 5, true) < light(HOUR * 6, true) && light(HOUR * 6, true) < light(HOUR * 8, true));
        assert!(light(HOUR * 19, true) > light(HOUR * 21, true));
        // The moon goes round once a month, full in the middle, and a full
        // moon's night is brighter than a new one's.
        assert_eq!(moon(0), 0);
        assert_eq!(moon(MONTH / 2), 4);
        assert_eq!(moon(MONTH), 0);
        assert!(light(MONTH / 2, true) > light(0, true));
        assert_eq!(time_words(HOUR * 12), "midday");
        assert_eq!(time_words(DAWN_TICK), "dawn");
        assert_eq!(moon_words(MONTH / 2), "a full moon");
    }
}
