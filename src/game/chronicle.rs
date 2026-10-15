//! The year's chronicle: at each new year the fortress's year is told as a
//! short story, titled after its biggest event.

use super::*;
use serde::{Deserialize, Serialize};

/// What happened this year, gathered as it happens.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct YearLog {
    pub deaths: Vec<String>,
    pub births: u32,
    pub arrivals: u32,
    pub kills: u32,
    pub fires: u32,
    pub autumn_fire: bool,
    pub festivals: Vec<String>,
    pub crimes: u32,
    pub dragon: bool,
    pub beast: Option<String>,
    pub siege: Option<String>,
    pub cave_in: bool,
    pub artifact: Option<String>,
    pub hero: Option<String>,
    #[serde(default)]
    pub ruin: Option<String>,
    #[serde(default)]
    pub undead: bool,
    #[serde(default)]
    pub monument: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chronicle {
    pub year: i32,
    pub title: String,
    pub lines: Vec<String>,
}

fn list(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

impl YearLog {
    /// The year's name, from the biggest thing that happened in it.
    pub fn title(&self) -> String {
        if self.dragon {
            return "The Year of the Dragon".into();
        }
        if self.undead {
            return "The Year the Dead Walked".into();
        }
        if let Some(b) = &self.beast {
            return format!("The Year of the {}", cap(b));
        }
        if let Some(m) = &self.monument {
            return format!("The Year of {}", cap(m));
        }
        if let Some(r) = &self.ruin {
            return format!("The Year of {}", cap(r));
        }
        if let Some(h) = &self.hero {
            return format!("The Year of {}", h);
        }
        if let Some(c) = &self.siege {
            return format!("The Year of War with the {}", c);
        }
        if self.cave_in {
            return "The Year the Ceiling Fell".into();
        }
        if self.deaths.len() >= 3 {
            return "The Year of Sorrow".into();
        }
        if let Some(a) = &self.artifact {
            return format!("The Year of {}", a);
        }
        if self.autumn_fire {
            return "The Year of Burning Leaves".into();
        }
        if self.fires > 0 {
            return "The Year of Smoke".into();
        }
        if self.births >= 2 {
            return "The Year of Cradles".into();
        }
        if !self.festivals.is_empty() {
            return "The Year of Feasting".into();
        }
        "The Year of Quiet Stone".into()
    }
}

impl Game {
    #[cfg(test)]
    pub fn write_chronicle_for_test(&mut self) {
        self.write_chronicle();
    }

    /// At the turn of the year: tell the year that ended, and start a new log.
    pub(super) fn write_chronicle(&mut self) {
        if self.tick < YEAR / 2 {
            return;
        }
        let log = std::mem::take(&mut self.year_log);
        let year = self.start_year + (self.tick / YEAR) as i32 - 1;
        let fort = self.fortress_name.clone();
        let mut lines = vec![format!("In the year {}, {} numbered {} souls and was worth ☼{}.", year, fort, self.alive_count(), self.wealth())];
        if log.arrivals > 0 {
            lines.push(format!("{} migrants came seeking a home in the mountain.", log.arrivals));
        }
        if log.births > 0 {
            lines.push(if log.births == 1 { "A child was born.".to_string() } else { format!("{} children were born.", log.births) });
        }
        if !log.festivals.is_empty() {
            let mut f = log.festivals.clone();
            f.dedup();
            lines.push(format!("We kept the {}, and made merry.", list(&f)));
        }
        if let Some(c) = &log.siege {
            lines.push(format!("The {} came against us in war.", c));
        }
        if log.kills > 0 {
            lines.push(format!("Our dwarves and our traps slew {} foe{}.", log.kills, if log.kills == 1 { "" } else { "s" }));
        }
        if log.fires > 0 {
            lines.push(format!("Fire broke out {} time{}.", log.fires, if log.fires == 1 { "" } else { "s" }));
        }
        if log.crimes > 0 {
            lines.push(format!("{} crime{} went on the record.", log.crimes, if log.crimes == 1 { "" } else { "s" }));
        }
        // The great deeds and events, as the legends tell them.
        let told: Vec<String> = self.overworld.events.iter().filter(|e| e.year == year && e.text.contains(&fort)).map(|e| e.text.clone()).collect();
        let skip = told.len().saturating_sub(10);
        lines.extend(told.into_iter().skip(skip));
        if log.deaths.is_empty() {
            lines.push("No dwarf of the fortress died. May the next year be as kind.".into());
            self.milestone("nodeath", "A whole year without a death");
        } else {
            lines.push(format!("We mourn {}.", list(&log.deaths)));
        }
        let title = log.title();
        self.msg(&format!("A new year dawns. The chronicler has written of {} (c).", title.to_lowercase().replacen("the ", "", 1)), (230, 210, 150));
        self.chronicles.push(Chronicle { year, title, lines });
        self.chronicle_ready = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::festivals::FestivalKind;

    #[test]
    fn titles_follow_the_biggest_event() {
        let mut l = YearLog::default();
        assert_eq!(l.title(), "The Year of Quiet Stone");
        l.festivals.push("harvest feast".into());
        assert_eq!(l.title(), "The Year of Feasting");
        l.fires = 2;
        l.autumn_fire = true;
        assert_eq!(l.title(), "The Year of Burning Leaves");
        l.deaths = vec!["A".into(), "B".into(), "C".into()];
        assert_eq!(l.title(), "The Year of Sorrow");
        l.siege = Some("Fellowship of Silver Lanterns".into());
        assert_eq!(l.title(), "The Year of War with the Fellowship of Silver Lanterns");
        l.dragon = true;
        assert_eq!(l.title(), "The Year of the Dragon");
    }

    #[test]
    fn a_year_is_written_down_when_it_ends() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.start_festival(FestivalKind::Harvest);
        let mut d = std::mem::take(&mut g.dwarves[1]);
        g.kill(&mut d, "was crushed");
        g.dwarves[1] = d;
        assert_eq!(g.year_log.festivals.len(), 1);
        assert_eq!(g.year_log.deaths.len(), 1);
        g.tick = YEAR;
        g.write_chronicle();
        let c = g.chronicles.last().expect("no chronicle");
        assert_eq!(c.year, g.start_year);
        assert!(c.lines.iter().any(|l| l.contains("harvest feast")), "{:?}", c.lines);
        assert!(c.lines.iter().any(|l| l.starts_with("We mourn")), "{:?}", c.lines);
        assert!(g.chronicle_ready);
        assert_eq!(g.year_log, YearLog::default(), "a fresh log for the new year");
    }
}
