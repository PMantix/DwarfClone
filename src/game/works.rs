//! Works of art: dwarves compose poems, tunes and dances in their people's
//! forms (`arts.rs`), each with a name and a subject from the fortress's
//! legends, and now and then devise a new form that the neighbouring
//! peoples take up. Bards and the fortress's own musicians perform them.
//!
//! Every roll is on the culture's own stream (`culture.rs`).

use super::*;
use crate::arts::{FormKind, Form};
use crate::creature::Creature;
use crate::dwarf::NeedKind;

/// A named work in one of the forms.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Work {
    /// "The Lay of Ironhold".
    pub title: String,
    /// The form's name ("zanost").
    pub form: String,
    pub kind: FormKind,
    pub author: String,
    pub author_id: u32,
    pub year: i32,
    /// What it tells of ("how the goblins came against Ironhold").
    pub about: String,
    /// Times it has been performed.
    #[serde(default)]
    pub performed: u32,
}

pub fn kind_word(k: FormKind) -> &'static str {
    match k {
        FormKind::Poetry => "poem",
        FormKind::Music => "tune",
        FormKind::Dance => "dance",
    }
}

const POEM_TITLES: [&str; 5] = ["The Lay of", "Verses on", "The Lament for", "The Boast of", "Words for"];
const TUNE_TITLES: [&str; 4] = ["The Air of", "The March of", "A Tune for", "The Hymn of"];
const DANCE_TITLES: [&str; 4] = ["The Dance of", "The Stamp of", "The Reel of", "The Steps of"];

/// What a legend is about, for a title, and how it went, for the telling:
/// ("Ironhold", "the goblins came against Ironhold").
pub fn subject_of(event: &str, fort: &str) -> (String, String) {
    const OPENERS: [&str; 10] = ["The", "A", "An", "After", "In", "On", "At", "When", "Word", "Fire"];
    let words: Vec<&str> = event.split(' ').collect();
    let first = words.first().copied().unwrap_or("");
    let subject = if OPENERS.contains(&first) {
        if event.contains(fort) { fort.to_string() } else { format!("the {}", words.get(1).copied().unwrap_or("Ages").trim_end_matches(|ch: char| !ch.is_alphanumeric())) }
    } else {
        // The name at the front: capitalised words, up to a comma.
        let mut name: Vec<&str> = Vec::new();
        for w in &words {
            if !w.chars().next().map_or(false, |c| c.is_uppercase()) {
                break;
            }
            name.push(w);
            if w.ends_with(',') {
                break;
            }
        }
        if name.is_empty() { fort.to_string() } else { name.join(" ") }
    };
    let subject = subject.trim_end_matches(|ch: char| !ch.is_alphanumeric()).to_string();
    let mut how = event.trim_end_matches('.').to_string();
    if OPENERS.contains(&first) {
        how.replace_range(..1, &how[..1].to_lowercase());
    }
    (subject, how)
}

impl Game {
    /// A legend of the fortress's own, written by the culture: kept out of
    /// what the culture later draws its subjects from.
    pub(super) fn c_legend(&mut self, text: String) {
        self.legend(text);
        let n = self.overworld.events.len();
        self.culture.own.push(n.saturating_sub(1));
    }

    /// The fortress's legends (indices into the world's events), leaving out
    /// the culture's own entries.
    pub(super) fn fort_events(&self) -> Vec<usize> {
        let fort = &self.fortress_name;
        self.overworld.events.iter().enumerate().filter(|(i, e)| e.text.contains(fort.as_str()) && !self.culture.own.contains(i)).map(|(i, _)| i).collect()
    }

    /// Once a day: perhaps someone with an artist's soul composes, and the
    /// fortress's new forms travel to its neighbours.
    pub(super) fn works_daily(&mut self) {
        self.forms_spread();
        let cands: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.ghost && !d.insane && !d.is_child() && d.mood.is_none() && d.tantrum == 0 && d.personality.artistry >= 10
            })
            .collect();
        let base = self.knob("culture.compose", 0.006);
        for i in cands {
            let p = base * self.dwarves[i].personality.artistry as f64 / 50.0;
            if self.c_roll() < p {
                self.compose(i);
                return;
            }
        }
    }

    /// Dwarf `i` composes a work, sometimes in a form of their own devising.
    pub(super) fn compose(&mut self, i: usize) -> Option<usize> {
        let arts = self.our_arts();
        if arts.forms.is_empty() {
            return None;
        }
        let year = self.start_year + (self.tick / YEAR) as i32;
        let fort = self.fortress_name.clone();
        let name = self.dwarves[i].name.clone();
        let tick = self.tick;
        let form = if self.dwarves[i].personality.artistry >= 25 && self.c_roll() < self.knob("culture.invent", 0.15) {
            self.devise_form(i)
        } else {
            arts.forms[self.c_pick(arts.forms.len())].clone()
        };
        // The subject: what the composer lived through, else the fortress's
        // legends, else the world's.
        let lived = self.lived_through(self.dwarves[i].id);
        let ours = self.fort_events();
        let pool = if !lived.is_empty() && self.c_roll() < 0.7 { lived } else { ours };
        let event = if !pool.is_empty() {
            Some(pool[self.c_pick(pool.len())])
        } else if !self.overworld.events.is_empty() {
            Some(self.c_pick(self.overworld.events.len()))
        } else {
            None
        };
        let (subject, about) = match event {
            Some(e) => {
                let (s, h) = subject_of(&self.overworld.events[e].text, &fort);
                (s, format!("how {}", h))
            }
            None => (fort.clone(), format!("the halls of {}", fort)),
        };
        let titles: &[&str] = match form.kind {
            FormKind::Poetry => &POEM_TITLES,
            FormKind::Music => &TUNE_TITLES,
            FormKind::Dance => &DANCE_TITLES,
        };
        let mut title = format!("{} {}", titles[self.c_pick(titles.len())], subject);
        if self.culture.works.iter().any(|w| w.title == title) {
            let n = self.culture.works.iter().filter(|w| w.title.starts_with(&title)).count() + 1;
            title = format!("{} ({})", title, roman(n));
        }
        let w = Work { title: title.clone(), form: form.name.clone(), kind: form.kind, author: name.clone(), author_id: self.dwarves[i].id, year, about: about.clone(), performed: 0 };
        self.culture.works.push(w);
        let d = &mut self.dwarves[i];
        d.think(&format!("composed \"{}\"", title), -10, tick);
        self.count("culture.works", 1);
        self.msg(&format!("{} has composed \"{}\", a {} in the {} form, on {}.", name, title, kind_word(form.kind), form.name, about), (200, 180, 255));
        self.c_legend(format!("{} of {} composed \"{}\", a {} in the {} form.", name, fort, title, kind_word(form.kind), form.name));
        Some(self.culture.works.len() - 1)
    }

    /// Dwarf `i` devises a new form, kept with the world's arts.
    pub(super) fn devise_form(&mut self, i: usize) -> Form {
        let arts = self.our_arts();
        let kind = [FormKind::Poetry, FormKind::Music, FormKind::Dance][self.c_pick(3)];
        let civ = self.civ.unwrap_or(u32::MAX);
        let mut rng = self.c_rng().clone();
        let mut f = crate::arts::new_form(&mut rng, Race::Dwarf, civ, kind, &arts.instruments);
        while arts.forms.iter().chain(&self.overworld.arts.invented).any(|g| g.name == f.name) {
            f = crate::arts::new_form(&mut rng, Race::Dwarf, civ, kind, &arts.instruments);
        }
        self.culture.rng = Some(rng);
        let year = self.start_year + (self.tick / YEAR) as i32;
        let (name, fort) = (self.dwarves[i].name.clone(), self.fortress_name.clone());
        f.origin = Some(format!("{} of {}, in {}", name, fort, year));
        self.overworld.arts.invented.push(f.clone());
        let tick = self.tick;
        self.dwarves[i].think(&format!("devised the {}, a new {}", f.name, kind.name()), -15, tick);
        self.count("culture.forms", 1);
        self.msg(&format!("{} has devised a new {}: {}.", name, kind.name(), f.describe()), (255, 215, 120));
        self.c_legend(format!("{} of {} devised the {}, a new {}.", name, fort, f.name, kind.name()));
        f
    }

    /// The forms devised here travel: now and then a neighbour at peace takes one up.
    pub(super) fn forms_spread(&mut self) {
        let civ = self.civ.unwrap_or(u32::MAX);
        let ours: Vec<String> = self.overworld.arts.invented.iter().filter(|f| f.civ == civ).map(|f| f.name.clone()).collect();
        if ours.is_empty() {
            return;
        }
        let friends: Vec<u32> = self.neighbors.iter().filter(|n| !self.at_war_with(n) && n.civ != civ).map(|n| n.civ).collect();
        for name in ours {
            for &c in &friends {
                if self.overworld.arts.knows(c, &name) || self.c_roll() >= self.knob("culture.spread", 0.01) {
                    continue;
                }
                self.overworld.arts.adopted.push((c, name.clone()));
                let people = self.overworld.civ(c).name.clone();
                let fort = self.fortress_name.clone();
                self.count("culture.spread", 1);
                self.msg(&format!("Word comes that the {} have taken up the {}, devised in {}.", people, name, fort), (200, 180, 255));
                self.c_legend(format!("The {} took up the {}, devised in {}.", people, name, fort));
            }
        }
    }

    /// A work of the fortress's, of a kind (any kind but dance with `None`).
    pub(super) fn pick_work(&mut self, kind: Option<FormKind>) -> Option<usize> {
        let fit: Vec<usize> = (0..self.culture.works.len()).filter(|&i| kind.map_or(self.culture.works[i].kind != FormKind::Dance, |k| self.culture.works[i].kind == k)).collect();
        (!fit.is_empty()).then(|| fit[self.c_pick(fit.len())])
    }

    /// A work is performed: its author is glad to hear it.
    pub(super) fn performed(&mut self, w: usize) {
        self.culture.works[w].performed += 1;
        let (author, title) = (self.culture.works[w].author_id, self.culture.works[w].title.clone());
        let tick = self.tick;
        self.count("culture.performed", 1);
        if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == author && d.alive) {
            if !d.thoughts.iter().any(|t| t.text.starts_with("was glad to hear their") && tick < t.tick + DAY) {
                d.think(&format!("was glad to hear their \"{}\" performed", title), -8, tick);
            }
        }
    }

    /// A bard may perform one of the fortress's own works instead of a
    /// legend: true if they did.
    pub(super) fn bard_performs_work(&mut self, c: &Creature) -> bool {
        if self.culture.works.is_empty() || self.c_roll() >= 0.4 {
            return false;
        }
        let Some(w) = self.pick_work(None).or_else(|| self.pick_work(Some(FormKind::Dance))) else { return false };
        let work = self.culture.works[w].clone();
        let verb = if work.kind == FormKind::Dance { "dances" } else { "performs" };
        self.msg(&format!("{} the bard {} \"{}\" by {}, a {}, in the tavern.", c.title(), verb, work.title, work.author, work.form), (200, 180, 255));
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || d.asleep || self.zone_kind_at(d.pos) != Some(ZoneKind::Tavern) {
                continue;
            }
            let d = &mut self.dwarves[i];
            if d.id != work.author_id {
                d.think("enjoyed a bard's song", -6, tick);
            }
            d.satisfy(NeedKind::Party, 300);
        }
        self.performed(w);
        self.culture.stage = Some((c.pos, format!("{} performing \"{}\"", c.title(), work.title), tick + DAY / 8));
        true
    }

    /// The form a bard sings their legend in: one of their people's
    /// ("a zanost of the Fellowship of Lanterns"), by what the bard is.
    pub(super) fn bard_form(&mut self, c: &Creature) -> Option<String> {
        let desc = c.desc.clone().unwrap_or_default();
        let race = [Race::Dwarf, Race::Human, Race::Elf, Race::Goblin].into_iter().find(|r| {
            let s = match r {
                Race::Dwarf => "dwarf",
                Race::Human => "human",
                Race::Elf => "elf",
                Race::Goblin => "goblin",
            };
            desc.contains(s)
        })?;
        let civ = if race == Race::Dwarf && self.civ.is_some() {
            self.civ
        } else {
            self.neighbors.iter().find(|n| n.race == race).map(|n| n.civ).or_else(|| self.overworld.civs.iter().find(|c| c.race == race).map(|c| c.id))
        }?;
        let arts = self.overworld.civ_arts(civ);
        let forms: Vec<&Form> = arts.forms.iter().filter(|f| f.kind != FormKind::Dance).collect();
        if forms.is_empty() {
            return None;
        }
        let f = forms[self.c_pick(forms.len())].clone();
        Some(format!("a {} of the {}", f.name, self.overworld.civ(civ).name))
    }

    /// Works for the chronicle's page of a year.
    pub fn works_of_year(&self, year: i32) -> Vec<String> {
        self.culture.works.iter().filter(|w| w.year == year).map(|w| format!("\"{}\", a {} in the {} form by {}", w.title, kind_word(w.kind), w.form, w.author)).collect()
    }
}

fn roman(n: usize) -> &'static str {
    ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"].get(n.saturating_sub(1)).copied().unwrap_or("X+")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::visitors::BARD;

    fn artist(g: &mut Game) -> usize {
        let i = g.dwarves.iter().position(|d| !d.is_child()).unwrap();
        g.dwarves[i].personality.artistry = 50;
        i
    }

    #[test]
    fn a_dwarf_composes_a_named_work_and_it_goes_in_the_legends() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let i = artist(&mut g);
        g.knobs.set.insert("culture.invent".into(), 0.0);
        let w = g.compose(i).expect("no work");
        let work = g.culture.works[w].clone();
        assert_eq!(work.author, g.dwarves[i].name);
        assert!(g.our_arts().forms.iter().any(|f| f.name == work.form && f.kind == work.kind));
        assert!(work.about.starts_with("how ") && work.about.contains(&g.fortress_name), "{}", work.about);
        assert!(g.overworld.events.last().unwrap().text.contains(&format!("composed \"{}\"", work.title)));
        assert!(g.dwarves[i].thoughts.iter().any(|t| t.text.starts_with("composed")));
        // The culture's own entries are not subjects for more works.
        let n = g.overworld.events.len() - 1;
        assert!(!g.fort_events().contains(&n));
        assert_eq!(g.works_of_year(work.year).len(), 1);
        println!("{:?}", work);
    }

    #[test]
    fn a_legend_gives_a_title_its_subject() {
        let f = "Ironhold";
        assert_eq!(subject_of("Ashgim the cave horror emerged from the depths beneath Ironhold.", f).0, "Ashgim");
        assert_eq!(subject_of("Okgash Mogsk, general of the Horde, fell at Ironhold.", f).0, "Okgash Mogsk");
        let (s, how) = subject_of("After a siege of Ironhold, its overseer resolved to arm the soldiers.", f);
        assert_eq!(s, "Ironhold");
        assert!(how.starts_with("after a siege"));
        assert_eq!(subject_of("The Grim Shields killed the wolf.", f).0, "the Grim");
    }

    #[test]
    fn a_new_form_is_devised_and_spreads_to_the_neighbours() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let i = artist(&mut g);
        g.knobs.set.insert("culture.invent".into(), 1.0);
        let w = g.compose(i).unwrap();
        let form = g.culture.works[w].form.clone();
        let f = g.overworld.arts.invented.iter().find(|f| f.name == form).expect("a form devised").clone();
        assert!(f.origin.as_ref().unwrap().contains(&g.dwarves[i].name));
        assert!(g.our_arts().forms.iter().any(|x| x.name == form), "the fortress's people know it");
        assert!(g.log.iter().any(|l| l.text.contains("has devised a new")));
        // A neighbour at peace takes it up.
        assert!(g.neighbors.iter().any(|n| !g.at_war_with(n)), "no friendly neighbours in this world");
        g.knobs.set.insert("culture.spread".into(), 1.0);
        g.forms_spread();
        assert!(!g.overworld.arts.adopted.is_empty());
        let (c, _) = g.overworld.arts.adopted[0].clone();
        assert!(g.overworld.civ_arts(c).forms.iter().any(|x| x.name == form));
        assert!(g.overworld.events.iter().any(|e| e.text.contains(&format!("took up the {}", form))));
        // Only once per people.
        let n = g.overworld.arts.adopted.len();
        g.forms_spread();
        assert_eq!(g.overworld.arts.adopted.len(), n);
    }

    #[test]
    fn bards_sing_in_their_peoples_forms_and_perform_the_forts_works() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        assert!(g.add_zone(e.offset(-3, -3, 0), e.offset(3, 3, 0), ZoneKind::Tavern) > 0);
        let tiles: Vec<Pos> = g.zones.values().flat_map(|z| z.tiles.iter().copied()).filter(|&p| g.world.walkable(p)).collect();
        for (k, d) in g.dwarves.iter_mut().enumerate() {
            d.pos = tiles[k % tiles.len()];
            d.asleep = false;
        }
        let bard = g.spawn_visitor(e, BARD);
        let c = g.creatures[&bard].clone();
        let form = g.bard_form(&c).expect("a bard's people has forms");
        assert!(form.starts_with("a ") && form.contains(" of the "));
        // With a work of the fortress's to perform:
        let i = artist(&mut g);
        g.knobs.set.insert("culture.invent".into(), 0.0);
        let w = g.compose(i).unwrap();
        let title = g.culture.works[w].title.clone();
        let mut did = false;
        for _ in 0..30 {
            did |= g.bard_performs_work(&c);
        }
        assert!(did);
        assert!(g.culture.works[w].performed > 0);
        assert!(g.log.iter().any(|l| l.text.contains(&format!("\"{}\" by", title))));
        assert!(g.dwarves[i].thoughts.iter().any(|t| t.text.starts_with("was glad to hear their")));
    }
}
