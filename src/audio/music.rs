//! The composer: short generative pieces in a dwarven mood. Slow modal
//! melodies on plucked strings and low horns over a drone, phrased with
//! rests; a jig for feasts; war drums for a siege; a lament after deaths.
//! The season colours each piece (key, mode, tempo, instruments). Pieces
//! are separated by silence, so the music comes and goes.
//!
//! Composition is pure: `compose(mood, season, seed)` always writes the same
//! piece for the same arguments (the tests read the notes).

use super::dsp::Noise;

/// What the music is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Mood {
    /// No music.
    #[default]
    Silent,
    /// The title screen and the menus before a game.
    Title,
    /// A world being made.
    WorldGen,
    /// Peace in the fortress.
    Calm,
    /// A festival, a wedding, a victory feast or a full tavern.
    Festive,
    /// A siege coming or under way.
    War,
    /// Someone has died.
    Mourning,
    /// The arena.
    Battle,
}

impl Mood {
    #[cfg(test)]
    pub const ALL: [Mood; 7] = [Mood::Title, Mood::WorldGen, Mood::Calm, Mood::Festive, Mood::War, Mood::Mourning, Mood::Battle];

    /// Seconds of silence after a piece (lo, hi).
    pub fn gap(self) -> (f32, f32) {
        match self {
            Mood::Silent => (5.0, 5.0),
            Mood::Title => (8.0, 16.0),
            Mood::WorldGen => (6.0, 12.0),
            Mood::Calm => (45.0, 120.0),
            Mood::Festive => (5.0, 12.0),
            Mood::War => (4.0, 10.0),
            Mood::Mourning => (50.0, 120.0),
            Mood::Battle => (2.0, 4.0),
        }
    }

    /// Changing to this mood cuts the piece playing short.
    pub fn urgent(self) -> bool {
        matches!(self, Mood::War | Mood::Festive | Mood::Battle | Mood::Mourning | Mood::Title | Mood::WorldGen)
    }
}

/// The modes the melodies are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Dorian,
    Aeolian,
    Phrygian,
    /// For the brighter dances only.
    Mixolydian,
}

impl Scale {
    pub fn steps(self) -> [u8; 7] {
        match self {
            Scale::Dorian => [0, 2, 3, 5, 7, 9, 10],
            Scale::Aeolian => [0, 2, 3, 5, 7, 8, 10],
            Scale::Phrygian => [0, 1, 3, 5, 7, 8, 10],
            Scale::Mixolydian => [0, 2, 4, 5, 7, 9, 10],
        }
    }

    /// The MIDI note of a scale degree (0 = the root, 7 = an octave up).
    pub fn note(self, root: u8, degree: i32) -> u8 {
        let oct = degree.div_euclid(7);
        let step = self.steps()[degree.rem_euclid(7) as usize] as i32;
        (root as i32 + oct * 12 + step).clamp(0, 127) as u8
    }

    /// Whether a MIDI note belongs to this mode on `root`.
    #[cfg(test)]
    pub fn contains(self, root: u8, midi: u8) -> bool {
        let pc = (midi as i32 - root as i32).rem_euclid(12) as u8;
        self.steps().contains(&pc)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inst {
    Pluck,
    Horn,
    /// The horn an octave or two down, for war and lament.
    LowHorn,
    Drone,
    WarDrum,
    FrameDrum,
    Rim,
    Jingle,
    Bell,
}

impl Inst {
    /// Pitched, and so held to the mode.
    #[cfg(test)]
    pub fn melodic(self) -> bool {
        matches!(self, Inst::Pluck | Inst::Horn | Inst::LowHorn | Inst::Drone | Inst::Bell)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    /// When, in beats from the start of the piece.
    pub beat: f32,
    /// How long, in beats.
    pub len: f32,
    pub midi: u8,
    pub inst: Inst,
    /// 0..1.
    pub vel: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub mood: Mood,
    pub scale: Scale,
    pub root: u8,
    pub bpm: f32,
    /// Length in beats.
    pub beats: f32,
    /// Sorted by `beat`.
    pub notes: Vec<Note>,
}

impl Piece {
    pub fn secs_per_beat(&self) -> f32 {
        60.0 / self.bpm
    }

    pub fn seconds(&self) -> f32 {
        self.beats * self.secs_per_beat()
    }

    /// The notes on one instrument.
    #[cfg(test)]
    pub fn line(&self, inst: Inst) -> impl Iterator<Item = &Note> {
        self.notes.iter().filter(move |n| n.inst == inst)
    }
}

/// Key, mode and tempo for a mood in a season (0 spring .. 3 winter).
fn colour(mood: Mood, season: u8, n: &mut Noise) -> (Scale, u8, f32) {
    let s = season % 4;
    match mood {
        Mood::Calm => match s {
            // D dorian, a walking spring.
            0 => (Scale::Dorian, 50, n.range(66.0, 72.0)),
            // G dorian, warm and a touch quicker.
            1 => (Scale::Dorian, 55, n.range(70.0, 78.0)),
            // A aeolian: autumn darkens.
            2 => (Scale::Aeolian, 57, n.range(60.0, 66.0)),
            // E phrygian: winter in the deep.
            _ => (Scale::Phrygian, 52, n.range(52.0, 58.0)),
        },
        Mood::Festive => (if s == 1 || n.chance(0.4) { Scale::Mixolydian } else { Scale::Dorian }, [50, 55, 57, 52][s as usize], n.range(100.0, 112.0)),
        Mood::War => (if n.chance(0.6) { Scale::Phrygian } else { Scale::Aeolian }, [40, 43, 45, 40][s as usize], n.range(52.0, 58.0)),
        Mood::Mourning => (Scale::Aeolian, [45, 50, 45, 52][s as usize], n.range(46.0, 52.0)),
        Mood::Title => (Scale::Dorian, 50, 68.0),
        Mood::WorldGen => (Scale::Aeolian, 45, n.range(48.0, 54.0)),
        Mood::Battle => (Scale::Phrygian, 40, n.range(92.0, 100.0)),
        Mood::Silent => (Scale::Dorian, 50, 60.0),
    }
}

/// A melody line: a walk over scale degrees in rhythm cells, phrases
/// ending on a steady note, with rests between phrases.
struct Writer<'a> {
    n: &'a mut Noise,
    scale: Scale,
    root: u8,
    notes: Vec<Note>,
}

impl<'a> Writer<'a> {
    fn add(&mut self, beat: f32, len: f32, degree: i32, octave: i32, inst: Inst, vel: f32) {
        let midi = self.scale.note(self.root, degree + octave * 7);
        self.notes.push(Note { beat, len, midi, inst, vel });
    }

    /// One phrase of `bars` bars of `meter` beats from `start`, choosing
    /// rhythms from `cells`. Returns the degree it ended on.
    #[allow(clippy::too_many_arguments)]
    fn phrase(&mut self, start: f32, bars: u32, meter: f32, cells: &[&[f32]], mut deg: i32, lo: i32, hi: i32, octave: i32, inst: Inst, vel: f32, rest_chance: f32) -> i32 {
        let end = start + bars as f32 * meter;
        let mut t = start;
        while t < end - 0.01 {
            let cell = cells[self.n.pick(cells.len())];
            for &d in cell {
                if t >= end - 0.01 {
                    break;
                }
                let d = d.min(end - t);
                let last = t + d >= end - 0.01;
                if last {
                    // Close on the root, the third or the fifth.
                    let stable = [0, 2, 4, 7];
                    deg = *stable.iter().min_by_key(|&&s| (s - deg).abs()).unwrap();
                } else {
                    let step = [-2, -1, -1, 0, 1, 1, 2, 3, -3][self.n.pick(9)];
                    deg += step;
                    // Lean back toward the middle of the range.
                    if deg > hi {
                        deg = hi - 1;
                    }
                    if deg < lo {
                        deg = lo + 1;
                    }
                }
                if last || !self.n.chance(rest_chance) {
                    let accent = if (t - start) % meter < 0.01 { 1.0 } else { 0.8 };
                    let v = vel * accent * self.n.range(0.85, 1.0);
                    self.add(t, d, deg, octave, inst, v);
                }
                t += d;
            }
        }
        deg
    }
}

/// Write a piece.
pub fn compose(mood: Mood, season: u8, seed: u32) -> Piece {
    let mut n = Noise::new(seed ^ 0xD3A7_1C05);
    let (scale, root, bpm) = colour(mood, season, &mut n);
    let mut w = Writer { n: &mut n, scale, root, notes: Vec::new() };
    let beats = match mood {
        Mood::Calm | Mood::Title => calm(&mut w, mood, season),
        Mood::Festive => jig(&mut w),
        Mood::War => war(&mut w),
        Mood::Mourning => lament(&mut w),
        Mood::WorldGen => making(&mut w),
        Mood::Battle => battle(&mut w),
        Mood::Silent => 0.0,
    };
    let mut notes = w.notes;
    notes.sort_by(|a, b| a.beat.partial_cmp(&b.beat).unwrap());
    Piece { mood, scale, root, bpm, beats, notes }
}

const CALM_CELLS: &[&[f32]] = &[&[1.0, 1.0, 2.0], &[2.0, 1.0, 1.0], &[1.5, 0.5, 2.0], &[1.0, 1.0, 1.0, 1.0], &[3.0, 1.0], &[2.0, 2.0], &[0.5, 0.5, 1.0, 2.0]];
const SLOW_CELLS: &[&[f32]] = &[&[2.0, 2.0], &[3.0, 1.0], &[4.0], &[2.0, 1.0, 1.0]];

/// Strings over a drone, a horn answering now and then. 4/4.
fn calm(w: &mut Writer, mood: Mood, season: u8) -> f32 {
    let phrases = if mood == Mood::Title { 4 } else { 3 + w.n.pick(2) as u32 };
    let winter = mood == Mood::Calm && season % 4 == 3;
    let autumn = mood == Mood::Calm && season % 4 == 2;
    let rest_chance = if winter { 0.3 } else { 0.12 };
    let mut t = 0.0;
    let mut deg = 0;
    for p in 0..phrases {
        // The horn takes some phrases in autumn and winter, and answers in the title theme.
        let horn = (autumn || winter) && w.n.chance(0.45) || (mood == Mood::Title && p % 2 == 1);
        let (inst, cells, oct, vel) = if horn { (Inst::Horn, SLOW_CELLS, 0, 0.8) } else { (Inst::Pluck, CALM_CELLS, 1, 0.85) };
        deg = w.phrase(t, 2, 4.0, cells, deg, -1, 9, oct, inst, vel, rest_chance);
        // A low string marks the bar lines under the strings.
        if !horn {
            for bar in 0..2 {
                let d = if bar == 0 { 0 } else { 4 };
                w.add(t + bar as f32 * 4.0, 2.0, d, 0, Inst::Pluck, 0.5);
            }
        }
        // A winter bell, high and far.
        if winter && w.n.chance(0.5) {
            w.add(t + 6.0, 2.0, 4, 2, Inst::Bell, 0.4);
        }
        // The rest after the phrase: a bar or two of nothing but the drone.
        t += 8.0 + 4.0 * (1 + w.n.pick(2)) as f32;
    }
    let beats = t;
    w.add(0.0, beats - 2.0, 0, -1, Inst::Drone, 0.7);
    beats
}

const JIG_CELLS: &[&[f32]] = &[&[1.0, 1.0, 1.0], &[1.0, 1.0, 1.0], &[2.0, 1.0], &[1.0, 2.0], &[3.0]];

/// A jig in 6/8, counted in eighths: an A part and a B part, each played
/// twice, a hand drum and a jingle under it.
fn jig(w: &mut Writer) -> f32 {
    let bar = 6.0;
    let part = |w: &mut Writer, start: f32, from: i32| -> Vec<Note> {
        let before = w.notes.len();
        w.phrase(start, 4, bar, JIG_CELLS, from, 0, 9, 1, Inst::Pluck, 0.8, 0.05);
        w.notes.split_off(before)
    };
    let a = part(w, 0.0, 0);
    let b = part(w, 0.0, 4);
    let mut t = 0.0;
    for section in [&a, &a, &b, &b] {
        for nt in section.iter() {
            w.notes.push(Note { beat: nt.beat + t, ..*nt });
        }
        for i in 0..4 {
            let s = t + i as f32 * bar;
            w.notes.push(Note { beat: s, len: 1.0, midi: 0, inst: Inst::FrameDrum, vel: 0.9 });
            w.notes.push(Note { beat: s + 3.0, len: 1.0, midi: 0, inst: Inst::FrameDrum, vel: 0.6 });
            w.notes.push(Note { beat: s + 2.0, len: 1.0, midi: 0, inst: Inst::Rim, vel: 0.5 });
            w.notes.push(Note { beat: s + 5.0, len: 1.0, midi: 0, inst: Inst::Rim, vel: 0.5 });
            if i % 2 == 1 {
                w.notes.push(Note { beat: s + 3.0, len: 1.0, midi: 0, inst: Inst::Jingle, vel: 0.6 });
            }
            // Root and fifth on the strings below, one to a bar.
            let d = if i % 2 == 0 { 0 } else { 4 };
            w.add(s, 3.0, d, 0, Inst::Pluck, 0.45);
        }
        t += 4.0 * bar;
    }
    // A breath before the end.
    let beats = t + 3.0;
    w.add(0.0, t, 0, -1, Inst::Drone, 0.5);
    beats
}

/// Slow war drums and a low horn calling over a deep drone. 4/4.
fn war(w: &mut Writer) -> f32 {
    let bars = 16;
    for b in 0..bars {
        let s = b as f32 * 4.0;
        w.notes.push(Note { beat: s, len: 1.0, midi: 0, inst: Inst::WarDrum, vel: 1.0 });
        if b % 2 == 0 {
            w.notes.push(Note { beat: s + 2.5, len: 0.5, midi: 0, inst: Inst::WarDrum, vel: 0.55 });
            w.notes.push(Note { beat: s + 3.0, len: 1.0, midi: 0, inst: Inst::WarDrum, vel: 0.75 });
        } else {
            w.notes.push(Note { beat: s + 2.0, len: 1.0, midi: 0, inst: Inst::WarDrum, vel: 0.7 });
        }
        if b % 4 == 3 {
            w.notes.push(Note { beat: s + 3.5, len: 0.5, midi: 0, inst: Inst::FrameDrum, vel: 0.5 });
        }
    }
    // Horn calls: long notes on the root, the fifth and the dark second,
    // with bars of drums alone between them.
    let mut t = 4.0;
    while t < bars as f32 * 4.0 - 8.0 {
        let call: &[(i32, f32)] = match w.n.pick(3) {
            0 => &[(0, 2.0), (4, 2.0), (0, 4.0)],
            1 => &[(0, 1.0), (1, 1.0), (0, 2.0), (-3, 4.0)],
            _ => &[(4, 3.0), (3, 1.0), (1, 2.0), (0, 2.0)],
        };
        for &(d, len) in call {
            w.add(t, len * 0.95, d, 1, Inst::LowHorn, 0.85);
            t += len;
        }
        t += 4.0 * (1 + w.n.pick(2)) as f32;
    }
    let beats = bars as f32 * 4.0;
    w.add(0.0, beats - 1.0, 0, 0, Inst::Drone, 0.6);
    beats
}

/// A lament: a falling horn line, long notes and longer rests, a few
/// strings like tears. 4/4.
fn lament(w: &mut Writer) -> f32 {
    let mut t = 2.0;
    let mut deg = 4;
    for _ in 0..3 {
        let len = 4 + w.n.pick(2);
        for _ in 0..len {
            let d = [2.0, 2.0, 3.0, 4.0][w.n.pick(4)];
            w.add(t, d * 0.95, deg, 1, Inst::LowHorn, 0.7);
            t += d;
            // Mostly falling.
            deg += [-1, -1, -2, 1, 0][w.n.pick(5)];
            deg = deg.clamp(-2, 6);
        }
        w.add(t, 4.0, 0, 1, Inst::LowHorn, 0.6);
        deg = 4 + w.n.pick(2) as i32;
        t += 4.0;
        // Strings in the silence after.
        for k in 0..2 {
            if w.n.chance(0.7) {
                let d = [0, 2, 4, 7][w.n.pick(4)];
                w.add(t + k as f32 * 2.0, 2.0, d, 2, Inst::Pluck, 0.45);
            }
        }
        t += 4.0 + 2.0 * w.n.pick(2) as f32;
    }
    w.add(0.0, t - 2.0, 0, 0, Inst::Drone, 0.6);
    t
}

/// The world being made: a drone, and far bells and strings, unhurried.
fn making(w: &mut Writer) -> f32 {
    let beats = 96.0;
    let mut t = 2.0;
    while t < beats - 6.0 {
        let d = [0, 2, 3, 4, 6, 7, 9][w.n.pick(7)];
        if w.n.chance(0.5) {
            w.add(t, 3.0, d, 2, Inst::Bell, 0.5);
        } else {
            w.add(t, 3.0, d, 1, Inst::Pluck, 0.55);
        }
        t += [2.0, 3.0, 4.0, 6.0][w.n.pick(4)];
    }
    w.add(0.0, beats - 2.0, 0, 0, Inst::Drone, 0.8);
    w.add(16.0, 8.0, 4, 1, Inst::Horn, 0.4);
    w.add(56.0, 8.0, 2, 1, Inst::Horn, 0.4);
    beats
}

/// The arena: drums on every beat, the horn bellowing. 4/4.
fn battle(w: &mut Writer) -> f32 {
    let bars = 12;
    for b in 0..bars {
        let s = b as f32 * 4.0;
        for k in 0..4 {
            w.notes.push(Note { beat: s + k as f32, len: 1.0, midi: 0, inst: Inst::WarDrum, vel: if k == 0 { 1.0 } else { 0.6 } });
        }
        w.notes.push(Note { beat: s + 1.5, len: 0.5, midi: 0, inst: Inst::FrameDrum, vel: 0.5 });
        w.notes.push(Note { beat: s + 3.5, len: 0.5, midi: 0, inst: Inst::FrameDrum, vel: 0.6 });
        if b % 2 == 0 {
            let d = [0, 4, 1][w.n.pick(3)];
            w.add(s, 1.5, d, 1, Inst::LowHorn, 0.8);
            w.add(s + 2.0, 1.5, d, 1, Inst::LowHorn, 0.6);
        }
    }
    let beats = bars as f32 * 4.0;
    w.add(0.0, beats - 1.0, 0, 0, Inst::Drone, 0.5);
    beats
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moods() -> impl Iterator<Item = (Mood, u8, u32)> {
        Mood::ALL.into_iter().flat_map(|m| (0..4u8).flat_map(move |s| (1..6u32).map(move |seed| (m, s, seed))))
    }

    #[test]
    fn every_note_is_in_the_mode_and_inside_the_piece() {
        for (m, s, seed) in moods() {
            let p = compose(m, s, seed);
            assert!(!p.notes.is_empty(), "{:?} wrote nothing", m);
            assert!(p.beats > 8.0 && p.seconds() > 10.0 && p.seconds() < 150.0, "{:?} lasts {}s", m, p.seconds());
            for n in &p.notes {
                assert!(n.beat >= 0.0 && n.beat < p.beats && n.len > 0.0, "{:?}: {:?} outside {} beats", m, n, p.beats);
                assert!(n.vel > 0.0 && n.vel <= 1.0);
                if n.inst.melodic() {
                    assert!(p.scale.contains(p.root, n.midi), "{:?} {:?} on {}: {} not in the mode", m, p.scale, p.root, n.midi);
                    assert!((28..=100).contains(&n.midi), "{:?}: note {} out of range", m, n.midi);
                }
            }
            assert!(p.notes.windows(2).all(|w| w[0].beat <= w[1].beat), "notes sorted");
        }
    }

    #[test]
    fn slow_pieces_use_the_dwarven_modes() {
        for s in 0..4 {
            for seed in 1..6 {
                for m in [Mood::Calm, Mood::War, Mood::Mourning, Mood::Title, Mood::WorldGen] {
                    let p = compose(m, s, seed);
                    assert!(matches!(p.scale, Scale::Dorian | Scale::Aeolian | Scale::Phrygian), "{:?} in {:?}", m, p.scale);
                }
            }
        }
    }

    /// The melody stops to breathe: there are beats where no melodic note
    /// (but the drone) sounds.
    #[test]
    fn phrases_have_rests() {
        for (m, s, seed) in moods() {
            if matches!(m, Mood::Battle | Mood::Festive) {
                continue;
            }
            let p = compose(m, s, seed);
            let mut sounding = vec![false; p.beats.ceil() as usize + 1];
            for n in p.notes.iter().filter(|n| n.inst.melodic() && n.inst != Inst::Drone) {
                for b in n.beat.floor() as usize..(n.beat + n.len).ceil() as usize {
                    if b < sounding.len() {
                        sounding[b] = true;
                    }
                }
            }
            let silent = sounding.iter().filter(|s| !**s).count();
            assert!(silent >= 4, "{:?} season {} seed {}: only {} silent beats in {}", m, s, seed, silent, p.beats);
        }
    }

    #[test]
    fn war_has_slow_drums_and_the_feast_dances() {
        for s in 0..4 {
            let war = compose(Mood::War, s, 7);
            assert!(war.line(Inst::WarDrum).count() >= 16);
            assert!(war.bpm < 70.0);
            assert!(war.line(Inst::LowHorn).count() > 0);
            let jig = compose(Mood::Festive, s, 7);
            assert!(jig.bpm >= 100.0, "a jig moves");
            assert!(jig.line(Inst::FrameDrum).count() >= 16);
            assert_eq!(jig.line(Inst::WarDrum).count(), 0);
            let calm = compose(Mood::Calm, s, 7);
            assert_eq!(calm.line(Inst::WarDrum).count() + calm.line(Inst::FrameDrum).count(), 0, "peace has no drums");
            assert!(calm.bpm < 80.0);
            let lament = compose(Mood::Mourning, s, 7);
            assert!(lament.bpm < 55.0 && lament.line(Inst::LowHorn).count() > 4);
        }
    }

    #[test]
    fn the_seasons_colour_the_calm() {
        let keys: Vec<(Scale, u8)> = (0..4).map(|s| {
            let p = compose(Mood::Calm, s, 3);
            (p.scale, p.root)
        }).collect();
        assert_eq!(keys[0], (Scale::Dorian, 50));
        assert_eq!(keys[3].0, Scale::Phrygian);
        assert!(compose(Mood::Calm, 3, 3).bpm < compose(Mood::Calm, 1, 3).bpm, "winter is slower than summer");
    }

    #[test]
    fn composing_is_deterministic_and_seeds_differ() {
        assert_eq!(compose(Mood::Calm, 0, 42), compose(Mood::Calm, 0, 42));
        assert_ne!(compose(Mood::Calm, 0, 42).notes, compose(Mood::Calm, 0, 43).notes);
    }
}
