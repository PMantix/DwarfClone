//! Sound-based reports (O05, UI-06): what happens out of sight is heard,
//! not seen. A fight, a death, a cave-in or a fire whose place
//! (`LogEntry.at`) is off the screen or on another level comes into the
//! announcements as a sound from a direction ("Screams from the deep, far
//! to the east"), named for the place if it has a name; the words of the
//! report wait in the reports screen, and Enter takes the camera there.

use super::*;
use super::logkind::LogKind;

/// What the player is looking at: the map's corner, size and level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Earshot {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub z: i32,
}

impl Earshot {
    pub fn sees(&self, p: Pos) -> bool {
        p.z == self.z && p.x >= self.x && p.x < self.x + self.w && p.y >= self.y && p.y < self.y + self.h
    }
}

/// What kind of noise an announcement makes, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Noise {
    Scream,
    Steel,
    Rumble,
    Fire,
}

fn noise(e: &LogEntry) -> Option<Noise> {
    let t = &e.text;
    if t.starts_with("Cave-in") || t.contains("has come down") || t.contains("collapse") {
        return Some(Noise::Rumble);
    }
    if t.contains("burned") || t.contains("fire") || t.contains("Lightning") {
        return Some(Noise::Fire);
    }
    match e.kind() {
        LogKind::Death => Some(Noise::Scream),
        LogKind::Combat => Some(if t.contains("slain") || t.contains("has fallen") { Noise::Scream } else { Noise::Steel }),
        _ => None,
    }
}

/// "to the east", "far to the north-west": from the middle of the view.
fn bearing(from: (i32, i32), to: Pos) -> Option<String> {
    let (dx, dy) = (to.x - from.0, to.y - from.1);
    let dist = dx.abs().max(dy.abs());
    if dist < 12 {
        return None;
    }
    let ns = if dy * 5 < -dx.abs() * 2 { "north" } else if dy * 5 > dx.abs() * 2 { "south" } else { "" };
    let ew = if dx * 5 > dy.abs() * 2 { "east" } else if dx * 5 < -dy.abs() * 2 { "west" } else { "" };
    let dir = match (ns, ew) {
        ("", e) => e.to_string(),
        (n, "") => n.to_string(),
        (n, e) => format!("{}-{}", n, e),
    };
    Some(format!("{}to the {}", if dist > 50 { "far " } else { "" }, dir))
}

impl Game {
    /// The announcement `e` as a sound, if it happened out of `ear`'s
    /// sight and makes a noise; `None` to show it as it is.
    pub fn heard(&self, e: &LogEntry, ear: Earshot) -> Option<String> {
        let at = e.at?;
        if ear.sees(at) {
            return None;
        }
        let n = noise(e)?;
        let h = super::names::hash(&[e.tick, at.x as u64, at.y as u64, at.z as u64]);
        let what: &[&str] = match n {
            Noise::Scream => &["A scream", "Screams", "A cry, then silence,", "A long wail"],
            Noise::Steel => &["The clash of steel", "Shouts and the ring of blows", "The din of a fight", "Steel on steel"],
            Noise::Rumble => &["A deep rumble", "The crash of falling rock", "A roar of stone"],
            Noise::Fire => &["The crackle of flames", "A smell of smoke", "The roar of a fire"],
        };
        let mut s = what[(h % what.len() as u64) as usize].to_string();
        let dz = at.z - ear.z;
        let level = if dz <= -6 {
            Some("from the deep")
        } else if dz < 0 {
            Some("from below")
        } else if dz > 0 {
            Some("from above")
        } else {
            None
        };
        let place = self.place_at(at).map(|p| p.name.clone()).or_else(|| self.place_name(at));
        let mut parts: Vec<String> = Vec::new();
        if let Some(l) = level {
            parts.push(l.to_string());
        }
        match place {
            Some(p) => parts.push(format!("near {}", p)),
            None => {
                if let Some(b) = bearing((ear.x + ear.w / 2, ear.y + ear.h / 2), at) {
                    parts.push(b);
                }
            }
        }
        if parts.is_empty() {
            parts.push("nearby".into());
        }
        s.push(' ');
        s.push_str(&parts.join(", "));
        s.push('.');
        Some(s)
    }

    /// The newest sound heard from `ear` in the last two days, and where.
    pub fn last_heard(&self, ear: Earshot) -> Option<(Pos, String)> {
        let since = self.tick.saturating_sub(DAY * 2);
        self.log.iter().rev().take_while(|e| e.tick >= since).take(300).find_map(|e| self.heard(e, ear).map(|s| (e.at.unwrap(), s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn far_fights_and_deaths_are_heard() {
        let mut g = Game::new(5);
        let ear = Earshot { x: 0, y: 0, w: 40, h: 30, z: 10 };
        let far = Pos::new(150, 15, 10);
        g.msg_at("The goblin hacks Urist in the head!", (255, 120, 100), Some(far), LogKind::Combat);
        let s = g.heard(g.log.last().unwrap(), ear).expect("heard");
        assert!(s.contains("far to the east"), "{}", s);
        // Seen, it is told as it is; a quiet job makes no sound.
        let near = Pos::new(5, 5, 10);
        g.msg_at("The goblin hacks Urist in the head!", (255, 120, 100), Some(near), LogKind::Combat);
        assert_eq!(g.heard(g.log.last().unwrap(), ear), None);
        g.msg_at("Urist has made a door.", (200, 200, 200), Some(far), LogKind::Job);
        assert_eq!(g.heard(g.log.last().unwrap(), ear), None);
        // A death deep below.
        g.msg_at("Urist has bled to death.", (255, 60, 60), Some(Pos::new(10, 10, 2)), LogKind::Death);
        let s = g.heard(g.log.last().unwrap(), ear).unwrap();
        assert!(s.contains("from the deep"), "{}", s);
        assert_eq!(g.last_heard(ear).map(|x| x.0), Some(Pos::new(10, 10, 2)));
    }
}
