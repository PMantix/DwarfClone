//! The fort's mood at a glance (UI-01): one face per dwarf, coloured by how
//! they feel, and a skull for each dwarf who died this season. The status
//! bar draws it as the mood strip; the overseer looks down the same row of
//! faces once a season and says what it sees.

use super::overseer::Overseer;
use super::*;

/// One face in the strip.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub id: u32,
    pub glyph: char,
    pub color: (u8, u8, u8),
    /// 0 ecstatic .. 5 miserable, 6 out of their mind, 7 dead this season.
    pub rank: u8,
}

/// The glyphs: a smile, a frown, a dark face for a dwarf beside themselves,
/// a skull for the dead.
pub const SMILE: char = '☺';
pub const FROWN: char = '☹';
pub const WILD: char = '☻';
pub const SKULL: char = '☠';

/// How far down the scale a dwarf's stress puts them (the same steps as
/// `Dwarf::mood`).
pub fn rank_of(d: &Dwarf) -> u8 {
    if !d.alive {
        return 7;
    }
    if d.insane || d.melancholy || d.tantrum > 0 || crate::game::breakdowns::breakdown_mood(d).is_some() {
        return 6;
    }
    match d.stress {
        s if s < -150 => 0,
        s if s < -40 => 1,
        s if s < 80 => 2,
        s if s < 200 => 3,
        s if s < 350 => 4,
        _ => 5,
    }
}

impl Game {
    /// The strip, best moods first, then the long faces, then the dead of
    /// this season. Within a mood, by id, so faces don't jump about.
    pub fn mood_faces(&self) -> Vec<Face> {
        let season_start = self.tick - self.tick % SEASON;
        let mut v: Vec<Face> = self
            .dwarves
            .iter()
            .filter(|d| !d.ghost && (d.alive || d.died.map_or(false, |t| t >= season_start && t <= self.tick)))
            .map(|d| {
                let rank = rank_of(d);
                let glyph = match rank {
                    7 => SKULL,
                    6 => WILD,
                    0..=2 => SMILE,
                    _ => FROWN,
                };
                let color = if rank == 7 { (200, 200, 210) } else { d.mood().1 };
                Face { id: d.id, glyph, color, rank }
            })
            .collect();
        v.sort_by_key(|f| (f.rank, f.id));
        v
    }

    /// (long faces, living dwarves): unhappy or worse, or beside themselves.
    pub fn long_faces(&self) -> (usize, usize) {
        let alive: Vec<&Dwarf> = self.dwarves.iter().filter(|d| d.alive && !d.ghost).collect();
        (alive.iter().filter(|d| rank_of(d) >= 3).count(), alive.len())
    }

    /// Daily, from the overseer's round: on the tenth day of each season it
    /// looks down the row of faces, and if a third or more are long, or the
    /// season has taken two or more, it says so in its notes.
    pub(super) fn ov_mood_strip(&mut self, o: &mut Overseer) {
        if (self.tick % SEASON) / DAY != 10 {
            return;
        }
        let (long, alive) = self.long_faces();
        if alive >= 4 && long * 3 >= alive {
            let worst = self.dwarves.iter().filter(|d| d.alive && !d.ghost).max_by_key(|d| (rank_of(d), d.stress)).map(|d| d.name.split(' ').next().unwrap_or("").to_string());
            let text = match worst {
                Some(w) => format!("Looked down the row of faces: {} of {} long ones, {} the longest.", long, alive, w),
                None => format!("Looked down the row of faces: {} of {} long ones.", long, alive),
            };
            Self::note_ov(o, self.tick, text);
            self.count("long_faces_noted", 1);
        }
        let skulls = self.mood_faces().iter().filter(|f| f.rank == 7).count();
        if skulls >= 2 {
            Self::note_ov(o, self.tick, format!("{} skulls in the row of faces already this season.", skulls));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strip_has_a_face_per_dwarf_and_a_skull_per_death() {
        let mut g = Game::new(5);
        let n = g.alive_count();
        assert_eq!(g.mood_faces().len(), n);
        assert!(g.mood_faces().iter().all(|f| f.glyph == SMILE || f.glyph == FROWN));
        // One miserable, one dead.
        g.dwarves[0].stress = 500;
        let mut d = std::mem::take(&mut g.dwarves[1]);
        g.kill_for_test(&mut d, "was struck down");
        let dead = d.id;
        g.dwarves[1] = d;
        let faces = g.mood_faces();
        assert_eq!(faces.len(), n, "the dead stay in the strip as a skull this season");
        assert_eq!(faces.last().map(|f| (f.id, f.glyph)), Some((dead, SKULL)), "the skull comes last");
        let worst_living = &faces[faces.len() - 2];
        assert_eq!((worst_living.id, worst_living.glyph), (g.dwarves[0].id, FROWN));
        assert_eq!(g.long_faces(), (1, n - 1));
        // Next season the skull is gone.
        g.tick += SEASON;
        assert_eq!(g.mood_faces().len(), n - 1);
    }

    #[test]
    fn the_overseer_notes_a_row_of_long_faces() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        for d in g.dwarves.iter_mut() {
            d.stress = 260;
        }
        g.tick = SEASON + 10 * DAY;
        let mut o = g.overseer.take().unwrap();
        g.ov_mood_strip(&mut o);
        assert!(o.notes.iter().any(|n| n.1.starts_with("Looked down the row of faces")), "{:?}", o.notes);
        assert_eq!(g.annals.counters.get("long_faces_noted"), Some(&1));
    }
}
