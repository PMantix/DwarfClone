//! The gatehouse's defences in the legends (O04 steps 1c and 3).
//!
//! The defences take names as they are finished: the trap hall, named like
//! a mouth ("the Maw"), the pit under its bridge ("the Belly"), and the two
//! towers over the curtain's gate ("the Tusks"), each proclaimed through
//! `naming.rs` (`deed_at`), so the legends and the year's chronicle tell it.
//!
//! When a siege ends with half its army or more taken by them (caught or
//! killed in the hall, fallen into the pit, shot down from the towers), it
//! broke on the defences: the one that took the most is where it broke, the
//! siege is named for it ("the Siege of the Maw", "the Fall at the Belly",
//! `stories.rs: name_siege`), the defence keeps the deed in its name's
//! story, and the legends, a milestone, a spotlight and the siege's saga
//! tell it.
//!
//! Stage 3 of the gatehouse (`Overseer.gate_stage`): once the pit bridge
//! stands (or the rock refused a pit), and the fort has seen three sieges or
//! grown rich (a warlike or cautious overseer at once), the overseer raises
//! towers at the gate: the curtain wall's gate towers (`fortify.rs`), from
//! whose battlements its marksdwarves shoot down (`towers.rs`).

use super::fortify::WorkKind;
use super::naming::DeedKind;
use super::overseer::{Overseer, Style};
use super::personality::{NoteKind, Trait};
use super::pits::{PIT, PIT_DEPTH};
use super::places::PlaceKind;
use super::*;

/// One of the gatehouse's named defences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Defence {
    Hall,
    Pit,
    Towers,
}

impl Defence {
    pub const ALL: [Defence; 3] = [Defence::Hall, Defence::Pit, Defence::Towers];

    /// What it is called before it has a name.
    pub fn plain(self) -> &'static str {
        match self {
            Defence::Hall => "the trap hall",
            Defence::Pit => "the pit",
            Defence::Towers => "the gate towers",
        }
    }
}

/// Names for the gate towers: the trap hall is the mouth, these its tusks.
const TOWER_NAMES: [&str; 8] = ["the Tusks", "the Horns", "the Twin Fangs", "the Jaws", "the Grim Pair", "the Two Spears", "the Watchers", "the Iron Brothers"];

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn list(parts: &[String]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        n => format!("{} and {}", parts[..n - 1].join(", "), parts[n - 1]),
    }
}

fn in_box(a: Pos, b: Pos, p: Pos) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y) && p.z >= a.z.min(b.z) && p.z <= a.z.max(b.z)
}

impl Game {
    /// The curtain's gate towers, if both are finished.
    fn gate_towers(&self) -> Vec<usize> {
        let v: Vec<usize> = (0..self.works.list.len()).filter(|&i| self.works.list[i].kind == WorkKind::GateTower).collect();
        if v.len() >= 2 && v.iter().all(|&i| self.works.list[i].done()) {
            v
        } else {
            Vec::new()
        }
    }

    /// Where a defence stands, as a box (and its middle), if it does.
    pub(super) fn defence_box(&self, o: &Overseer, d: Defence) -> Option<(Pos, Pos, Pos)> {
        let base = o.base?;
        match d {
            // (The same boxes the gatehouse has always named them by.)
            Defence::Hall => o.did("traphall").then(|| (base.offset(8, 0, 0), base.offset(11, 1, 0), base.offset(10, 0, 0))),
            Defence::Pit => (!self.pits.decks.is_empty()).then(|| (base.offset(PIT[0], 0, 0), base.offset(PIT[1], 1, 0), base.offset(PIT[0], 0, 0))),
            Defence::Towers => {
                let t = self.gate_towers();
                let (first, last) = (t.first()?, t.last()?);
                let (a, b) = (self.works.list[*first].at, self.works.list[*last].at);
                let h = self.works.list[*first].height;
                let lo = Pos::new(a.x.min(b.x) - 2, a.y.min(b.y) - 2, a.z.min(b.z));
                let hi = Pos::new(a.x.max(b.x) + 2, a.y.max(b.y) + 2, a.z.max(b.z) + h);
                Some((lo, hi, Pos::new((a.x + b.x) / 2, (a.y + b.y) / 2, a.z)))
            }
        }
    }

    /// Which defence `p` is in (the pit: its bridge, or down in it).
    pub(super) fn defence_at(&self, o: &Overseer, p: Pos) -> Option<Defence> {
        Defence::ALL.into_iter().find(|&d| {
            self.defence_box(o, d).map_or(false, |(a, b, _)| {
                let a = if d == Defence::Pit { a.offset(0, 0, -PIT_DEPTH) } else { a };
                in_box(a, b, p)
            })
        })
    }

    /// The name a defence takes when it is finished (from hashes).
    pub(super) fn defence_name_for(&self, o: &Overseer, d: Defence) -> String {
        let base = o.base.unwrap_or(self.embark);
        match d {
            Defence::Hall => self.hall_name_for(base).to_string(),
            Defence::Pit => self.pits.name.clone().unwrap_or_else(|| self.pit_name_for(base).to_string()),
            Defence::Towers => {
                let h = super::names::hash(&[super::stories::hash(&self.fortress_name, 0), base.x as u64, base.y as u64, 0x746f_7772]);
                TOWER_NAMES[(h % TOWER_NAMES.len() as u64) as usize].to_string()
            }
        }
    }

    /// What the fort calls a defence now (named, renamed or plain).
    pub(super) fn defence_name(&self, o: &Overseer, d: Defence) -> String {
        self.defence_box(o, d)
            .and_then(|(a, b, _)| self.stories.places.iter().find(|pl| pl.kind == PlaceKind::Gate && pl.b.map_or(false, |pb| (pl.at, pb) == (a, b))).map(|pl| pl.name.clone()))
            .unwrap_or_else(|| d.plain().to_string())
    }

    /// The defence is finished: it takes its name, told as any place is.
    pub(super) fn name_defence(&mut self, o: &mut Overseer, d: Defence) {
        let Some((_, _, mid)) = self.defence_box(o, d) else { return };
        self.deed_at_defence(o, mid, d, DeedKind::Work, &[]);
    }

    /// Stage 3 of the gatehouse (from `ov_defences`): towers at the gate,
    /// once the pit is done with; and finished gate towers take a name.
    pub(super) fn ov_gate_towers(&mut self, o: &mut Overseer) {
        if o.style == Style::Reckless || o.base.is_none() {
            return;
        }
        let past_pit = (o.gate_stage == 2 && o.did("pitlever")) || (o.gate_stage == 1 && o.did("nopit"));
        if past_pit && self.towers_due(o) {
            o.gate_stage = 3;
            o.done.push("gatetowers".into());
            let text = if self.works.list.iter().any(|w| w.kind == WorkKind::GateTower) {
                "The towers over the yard's gate are the gatehouse's last part: marksdwarves on their battlements, shooting down.".to_string()
            } else {
                "Next for the gatehouse: towers at the gate. A curtain wall round the yard, two towers over its gate, battlements on top for the marksdwarves.".to_string()
            };
            Self::note_as(o, self.tick, NoteKind::Gate, text);
        }
        if !o.did("towersnamed") && !self.gate_towers().is_empty() {
            o.done.push("towersnamed".into());
            self.name_defence(o, Defence::Towers);
        }
    }

    /// Whether the towers are due: three sieges or a rich fort; a warlike
    /// or cautious overseer as soon as the pit is done with.
    pub(super) fn towers_due(&self, o: &Overseer) -> bool {
        let eager = o.style == Style::Warlord || o.has(Trait::Cautious);
        eager || self.annals.sieges.len() >= 3 || self.wealth() as f64 >= self.knob("gate.towers_wealth", 20000.0)
    }

    /// A dwarf has slain `c` (`credit_kill`): from a tower's top, it counts
    /// for the towers in the siege's toll.
    pub(super) fn tower_kill(&mut self, d: &Dwarf, c: &crate::creature::Creature) {
        if !c.siege || self.works.list.is_empty() {
            return;
        }
        let up = self.works.list.iter().any(|w| w.kind.is_tower() && w.covers(d.pos) && d.pos.z > w.at.z);
        if !up {
            return;
        }
        if let Some(s) = self.annals.sieges.iter_mut().rev().find(|s| s.ended.is_none()) {
            s.towered += 1;
        }
        self.count("m2.tower_kills", 1);
    }

    /// A siege is over (`on_siege_end`, after the tolls, before it is
    /// named): if the defences took half its army or more, it broke on
    /// them, and the one that took most is where.
    pub(super) fn defences_held(&mut self, i: usize) {
        let Some(s) = self.annals.sieges.get(i).cloned() else { return };
        if s.outcome == "the fortress fell" {
            return;
        }
        let Some(mut o) = self.overseer.take() else { return };
        let tolls = [(Defence::Hall, s.trapped + s.foes_caught), (Defence::Pit, s.fell), (Defence::Towers, s.towered)];
        let total: u32 = tolls.iter().map(|t| t.1).sum();
        let share = self.knob("gate.broke_share", 0.5);
        let (d, most) = tolls.iter().copied().filter(|t| self.defence_box(&o, t.0).is_some()).max_by_key(|t| t.1).unwrap_or((Defence::Hall, 0));
        if most == 0 || total < 2 || (total as f64) < share * s.size as f64 {
            self.overseer = Some(o);
            return;
        }
        // A defence that broke a siege before it was named takes its name now.
        let Some((a, b, mid)) = self.defence_box(&o, d) else {
            self.overseer = Some(o);
            return;
        };
        if !self.stories.places.iter().any(|pl| pl.kind == PlaceKind::Gate && pl.b.map_or(false, |pb| (pl.at, pb) == (a, b))) {
            self.deed_at_defence(&mut o, mid, d, DeedKind::Work, &[]);
        }
        let name = self.defence_name(&o, d);
        // The toll, told.
        let mut parts = Vec::new();
        for (k, n) in tolls.iter().copied().filter(|t| t.1 > 0 && self.defence_box(&o, t.0).is_some()) {
            let place = self.defence_name(&o, k);
            parts.push(match k {
                Defence::Hall => format!("{} were taken in {}", n, place),
                Defence::Pit => format!("{} fell into {}", n, place),
                Defence::Towers => format!("{} were shot down from {}", n, place),
            });
        }
        let fort = self.fortress_name.clone();
        let text = format!("The {} under {} broke upon {}: {}.", s.foe, s.leader, name, list(&parts));
        {
            let s = &mut self.annals.sieges[i];
            s.broke_on = Some(name.clone());
            s.broke_at = Some(mid);
        }
        self.legend(format!("The {} under {} broke upon {} of {}: {}.", s.foe, s.leader, name, fort, list(&parts)));
        self.msg_at(&text, (240, 200, 120), Some(mid), super::logkind::LogKind::Legend);
        self.spotlight("Broken on the defences", &text, Some(mid), (240, 200, 120));
        self.milestone("brokeondefences", "A siege broken on the defences");
        self.count("m2.broke_on_defences", 1);
        Self::note_as(&mut o, self.tick, NoteKind::Gate, format!("They broke upon {}. Let the songs say so.", name));
        // The defence keeps the deed in its story.
        let vets: Vec<u32> = self.dwarves.iter().filter(|x| x.alive && x.soldier).map(|x| x.id).take(8).collect();
        self.deed_at_defence(&mut o, mid, d, DeedKind::SiegeHeld, &vets);
        self.overseer = Some(o);
    }

    /// For the saga and the siege's name: the defence a siege broke on, if
    /// it is a proper name ("the Maw", not "the traps at the gate").
    pub(super) fn broke_on_name(s: &super::annals::SiegeNote) -> Option<String> {
        let d = s.broke_on.as_ref()?;
        let proper = d.starts_with("the ") && d[4..].chars().next().map_or(false, |c| c.is_uppercase());
        proper.then(|| d.clone())
    }
}

/// What `cap` does, for the other modules' lines ("The Maw held").
pub(super) fn capital(s: &str) -> String {
    cap(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Warlord fort with a trap hall laid and named, and its pit begun.
    fn defended() -> Game {
        let (found, passed) = super::super::overseer::fort_where(Style::Warlord, 5000, 12, 20, |g| g.overseer.as_ref().map_or(false, |o| o.did("linked")));
        let Some((_, mut g)) = found else { panic!("no Warlord fort from 5000 on had its gate linked by day 20 (passed over {:?})", passed) };
        let e = g.embark;
        for _ in 0..10 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        }
        for _ in 0..3 * DAY {
            if g.overseer.as_ref().unwrap().gate_stage >= 1 {
                break;
            }
            g.step();
        }
        let mut o = g.overseer.take().unwrap();
        assert_eq!(o.gate_stage, 1, "no trap hall");
        let base = o.base.unwrap();
        g.pits.decks = PIT.iter().flat_map(|&dx| (0..2).map(move |dy| base.offset(dx, dy, 0))).collect();
        g.pits.name = Some(g.pit_name_for(base).to_string());
        g.name_defence(&mut o, Defence::Pit);
        g.overseer = Some(o);
        g
    }

    /// O04 step 3: the trap hall and the pit are named through `deed_at`,
    /// told in the legends and the year's chronicle; a siege the pit took
    /// most of breaks upon it, is named for it, and the pit keeps the deed.
    #[test]
    fn a_siege_broken_on_the_traps_is_legend() {
        let mut g = defended();
        let o = g.overseer.clone().unwrap();
        let hall = g.defence_name(&o, Defence::Hall);
        let pit = g.defence_name(&o, Defence::Pit);
        assert!(hall != "the trap hall" && pit != "the pit", "{} / {}", hall, pit);
        assert!(g.overworld.events.iter().any(|l| l.text.contains(&format!("the pit was named {}", pit))), "no legend of the pit's name");
        assert!(g.year_log.named.iter().any(|l| l.contains(&hall)) && g.year_log.named.iter().any(|l| l.contains(&pit)), "{:?}", g.year_log.named);
        // A siege: three taken in the hall, five into the pit, of ten.
        g.note_siege("goblins", 10, "Snodub Gutripper");
        let i = g.annals.sieges.len() - 1;
        {
            let s = &mut g.annals.sieges[i];
            s.trapped = 3;
            s.fell = 5;
            s.foes_slain = 6;
            s.ended = Some(g.tick);
            s.outcome = "routed".into();
        }
        g.on_siege_end(i);
        let s = g.annals.sieges[i].clone();
        assert_eq!(s.broke_on.as_deref(), Some(pit.as_str()), "it broke on the pit");
        let said = format!("The goblins under Snodub Gutripper broke upon {}: 3 were taken in {} and 5 fell into {}.", pit, hall, pit);
        assert!(g.log.iter().any(|l| l.text == said), "nobody said: {}", said);
        assert!(g.overworld.events.iter().any(|l| l.text.contains(&format!("broke upon {} of {}", pit, g.fortress_name))));
        assert!(g.milestones.iter().any(|m| m.0 == "brokeondefences"));
        // Named for it: "the Siege of the Belly", "the Fall at the Belly"...
        let e = g.stories.episodes.iter().find(|e| e.siege == Some(i)).cloned().expect("the siege has a name");
        assert!(e.name.ends_with(&pit), "{}", e.name);
        // The pit keeps the deed.
        let o = g.overseer.clone().unwrap();
        let (a, _, _) = g.defence_box(&o, Defence::Pit).unwrap();
        let pl = g.place_at(a).unwrap();
        assert_eq!(pl.name, pit);
        assert!(pl.deed.contains("Snodub Gutripper"), "{}", pl.deed);
        // ...and the chronicle tells it.
        g.tick = YEAR + 5;
        g.write_chronicle_for_test();
        let c = g.chronicles.last().expect("a chronicle");
        assert!(c.lines.iter().any(|l| l.contains(&e.name)), "{:?}", c.lines);
        assert!(c.lines.iter().any(|l| l.contains(&format!("named {}", pit))), "{:?}", c.lines);
    }

    /// A siege the defences barely touched did not break on them.
    #[test]
    fn a_siege_that_beat_the_traps_is_not_theirs() {
        let mut g = defended();
        g.note_siege("goblins", 12, "Snodub Gutripper");
        let i = g.annals.sieges.len() - 1;
        {
            let s = &mut g.annals.sieges[i];
            s.trapped = 2;
            s.fell = 1;
            s.ended = Some(g.tick);
            s.outcome = "withdrew".into();
        }
        g.defences_held(i);
        assert!(g.annals.sieges[i].broke_on.is_none());
        assert!(!g.milestones.iter().any(|m| m.0 == "brokeondefences"));
    }

    /// O04 step 1c: past the pit, a warlike overseer goes on to towers at
    /// the gate, and the curtain wall that carries them is due at once.
    #[test]
    fn towers_rise_at_the_yard() {
        let mut g = defended();
        let mut o = g.overseer.take().unwrap();
        o.gate_stage = 2;
        o.done.push("pitlever".into());
        g.ov_gate_towers(&mut o);
        assert_eq!(o.gate_stage, 3);
        assert!(o.notes.iter().any(|n| n.1.contains("towers at the gate")), "{:?}", o.notes.last());
        // The curtain that carries them is due, day and wealth aside...
        assert!(g.curtain_due(&o), "the curtain isn't due at day {}", g.tick / DAY);
        g.overseer = Some(o);
        // ...and laid out on the overseer's next round.
        for _ in 0..2 * DAY {
            g.step();
            if g.works.laid {
                break;
            }
        }
        assert!(g.works.laid, "the curtain wasn't laid out");
        // Finished, the gate towers take a name.
        if g.works.list.iter().filter(|w| w.kind == WorkKind::GateTower).count() >= 2 {
            g.raise_works_now();
            let mut o = g.overseer.take().unwrap();
            g.ov_gate_towers(&mut o);
            let name = g.defence_name(&o, Defence::Towers);
            assert!(TOWER_NAMES.contains(&name.as_str()), "{}", name);
            assert!(g.log.iter().any(|l| l.text.contains(&format!("is named {}", name))));
            g.overseer = Some(o);
        }
    }
}
