//! A dwarf's life story (UI-05, UI-08): who they are, told from their
//! thoughts, family, friends and grudges, wounds, deeds, what the chronicle
//! and the legends say of them, and how they died; and their looks (beard,
//! hair, scars, lost limbs, gear) for the portrait on the Life page. When a
//! dwarf dies the overseer remembers them in a line, and the year's chronicle
//! keeps it.

use super::overseer::Overseer;
use super::*;
use crate::dwarf::Thought;

/// How a dwarf looks: fixed by who they are (nothing random is drawn).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Looks {
    /// 0 long, 1 braided, 2 forked, 3 short and bristly.
    pub beard: u8,
    /// 0 bald, 1 wild, 2 combed, 3 a topknot.
    pub hair: u8,
    pub hair_color: (u8, u8, u8),
    pub skin: (u8, u8, u8),
}

const HAIR: [(u8, u8, u8); 6] = [(150, 75, 30), (200, 150, 60), (90, 50, 30), (40, 30, 25), (190, 60, 30), (230, 200, 120)];
const SKIN: [(u8, u8, u8); 4] = [(235, 190, 150), (210, 160, 120), (170, 120, 85), (240, 205, 175)];

fn mix(id: u32, name: &str) -> u32 {
    name.bytes().fold(id.wrapping_mul(2654435761), |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

/// A dwarf's looks. Hair greys past 120 years.
pub fn looks(d: &Dwarf) -> Looks {
    let h = mix(d.id, &d.name);
    let old = d.age.map_or(false, |a| a >= 120);
    Looks {
        beard: (h % 4) as u8,
        hair: (h / 4 % 4) as u8,
        hair_color: if old { (200, 200, 205) } else { HAIR[(h / 16 % 6) as usize] },
        skin: SKIN[(h / 96 % 4) as usize],
    }
}

/// A scar shows on the face: the head has been hurt, or the history says
/// it was struck there.
pub fn face_scar(d: &Dwarf) -> bool {
    let i = crate::body::Part::Head.idx();
    d.body.hp[i] < d.body.max[i] || d.med.iter().any(|(_, t)| t.contains("head"))
}

fn first_name(name: &str) -> &str {
    name.split(' ').next().unwrap_or(name)
}

fn season_of(g: &Game, tick: u64) -> String {
    format!("{} of {}", SEASONS[((tick % YEAR) / SEASON) as usize], g.start_year as u64 + tick / YEAR)
}

impl Game {
    /// The story of a life, in paragraphs.
    pub fn life_story(&self, d: &Dwarf) -> Vec<String> {
        let me = first_name(&d.name);
        let they = match d.female {
            Some(true) => "she",
            Some(false) => "he",
            None => "they",
        };
        let name_of = |id: u32| self.dwarves.iter().find(|o| o.id == id).map(|o| o.name.clone());
        let mut out = Vec::new();
        // Who they are.
        let mut who = d.name.clone();
        if let Some(e) = &d.epithet {
            who.push_str(&format!(", called {},", e));
        }
        let age = d.age.map_or(String::new(), |a| format!("{}-year-old ", a));
        let post = match (d.noble, self.position_of(d.id)) {
            (Some(n), _) => format!(" and {}", n.title()),
            (None, Some(p)) => format!(" and {}", p.title()),
            _ => String::new(),
        };
        let what = format!("{}{}", age, d.profession().to_lowercase());
        // "an engraver", "an 8-year-old", "an 18-year-old".
        let vowel = what.starts_with(|c| "aeiou8".contains(c)) || what.starts_with("11-") || what.starts_with("18-");
        let a = if vowel { "an" } else { "a" };
        let mut s = format!("{} {} {} {}{} of {}.", who, if d.alive { "is" } else { "was" }, a, what, post, self.fortress_name);
        let traits = d.personality.describe();
        if !traits.is_empty() {
            let t: Vec<&str> = traits.iter().take(3).copied().collect();
            let joined = match t.len() {
                1 => t[0].to_string(),
                n => format!("{} and {}", t[..n - 1].join(", "), t[n - 1]),
            };
            s.push_str(&format!(" {} {}.", cap(they), joined));
        }
        s.push_str(&format!(" {} likes {}.", cap(they), d.likes_mat.name()));
        out.push(s);
        // Family and friends.
        let mut fam = Vec::new();
        if let Some((m, f)) = d.parents {
            if let (Some(m), Some(f)) = (name_of(m), name_of(f)) {
                fam.push(format!("{} was born here to {} and {}.", me, m, f));
            }
        }
        if let Some(sp) = d.spouse.and_then(|id| self.dwarves.iter().find(|o| o.id == id)) {
            fam.push(if sp.alive || !d.alive { format!("{} married {}.", me, sp.name) } else { format!("{} married {}, and was widowed.", me, sp.name) });
        }
        let kids: Vec<String> = self.dwarves.iter().filter(|o| o.family_of(d.id) && o.parents.map_or(false, |(m, f)| m == d.id || f == d.id)).map(|o| first_name(&o.name).to_string()).collect();
        if !kids.is_empty() {
            fam.push(format!("{} raised {}.", cap(they), list(&kids)));
        }
        let mut rel: Vec<(u32, i32)> = d.relations.iter().copied().filter(|r| r.1 >= 70 || r.1 <= -40).collect();
        rel.sort_by_key(|r| std::cmp::Reverse(r.1.abs()));
        let friends: Vec<String> = rel.iter().filter(|r| r.1 > 0).take(3).filter_map(|r| name_of(r.0)).map(|n| first_name(&n).to_string()).collect();
        let foes: Vec<String> = rel.iter().filter(|r| r.1 < 0).take(2).filter_map(|r| name_of(r.0)).map(|n| first_name(&n).to_string()).collect();
        if !friends.is_empty() {
            fam.push(format!("{} closest friend{} {}.", cap(if they == "they" { "their" } else if they == "she" { "her" } else { "his" }), match (friends.len() == 1, d.alive) {
                (true, true) => " is",
                (true, false) => " was",
                (false, true) => "s are",
                (false, false) => "s were",
            }, list(&friends)));
        }
        if !foes.is_empty() {
            fam.push(format!("{} {} abide {}.", cap(they), if d.alive { "cannot" } else { "could not" }, list(&foes)));
        }
        if !fam.is_empty() {
            out.push(fam.join(" "));
        }
        // Deeds.
        let mut deeds = Vec::new();
        if let Some(deed) = &d.deed {
            deeds.push(format!("{} {}.", me, deed));
        }
        if d.kills > 0 {
            deeds.push(format!("{} slew {} foe{}.", cap(they), d.kills, if d.kills == 1 { "" } else { "s" }));
        }
        let wins = d.duels_won + d.bouts_won + d.contests_won;
        if wins > 0 {
            deeds.push(if wins == 1 { format!("{} won a duel, bout or contest.", cap(they)) } else { format!("{} won {} duels, bouts and contests.", cap(they), wins) });
        }
        let best = LABORS.iter().enumerate().max_by_key(|(i, _)| d.skills[*i]).filter(|(i, _)| d.skills[*i] >= 1200);
        if let Some((i, l)) = best {
            let rank = crate::dwarf::skill_rank(d.skills[i]).to_lowercase();
            let a = if rank.starts_with(|c| "aeiou".contains(c)) { "an" } else { "a" };
            deeds.push(format!("{} {} {} {} {}.", cap(they), if d.alive { "is" } else { "was" }, a, rank, l.profession().to_lowercase()));
        }
        // The chronicle and the legends.
        for c in &self.chronicles {
            for l in c.lines.iter().filter(|l| l.contains(&d.name)) {
                deeds.push(format!("In {} the chronicle wrote: \"{}\"", c.year, l));
            }
        }
        for e in self.overworld.events.iter().filter(|e| e.text.contains(&d.name)).take(4) {
            if !deeds.iter().any(|x| x.contains(&e.text)) {
                deeds.push(format!("The legends of {} tell: {}", e.year, e.text));
            }
        }
        if !deeds.is_empty() {
            out.push(deeds.join(" "));
        }
        // Memories, the strongest of each season, oldest first.
        let mut seasons: Vec<(u64, &Thought)> = Vec::new();
        for t in &d.thoughts {
            let s = t.tick / SEASON;
            match seasons.iter_mut().find(|x| x.0 == s) {
                Some(x) if t.stress.abs() > x.1.stress.abs() => x.1 = t,
                Some(_) => {}
                None => seasons.push((s, t)),
            }
        }
        seasons.sort_by_key(|x| x.0);
        let mem: Vec<String> = seasons.iter().rev().take(6).rev().map(|(_, t)| format!("In {} {} {}.", season_of(self, t.tick), me, t.text)).collect();
        if !mem.is_empty() {
            out.push(mem.join(" "));
        }
        // Wounds.
        let mut hurt: Vec<String> = d.med.iter().rev().take(4).rev().map(|(t, what)| format!("{} was {} ({}).", cap(they), what, self.short_date(*t).trim())).collect();
        let lost: Vec<&str> = crate::body::PARTS.iter().filter(|p| d.body.lost[p.idx()]).map(|p| p.name()).collect();
        if !lost.is_empty() {
            hurt.push(format!("{} lost {} {}.", cap(they), if they == "she" { "her" } else if they == "he" { "his" } else { "their" }, list(&lost.iter().map(|s| s.to_string()).collect::<Vec<_>>())));
        }
        if !hurt.is_empty() {
            out.push(hurt.join(" "));
        }
        // The end: the death scene (familytree.rs) and where they lie.
        if !d.alive {
            let mut end = self.death_scene(d);
            if let Some(rest) = self.resting_place(d) {
                end.push_str(&format!(" {} {}.", cap(they), rest));
            }
            out.push(end);
        } else if out.len() <= 2 && d.thoughts.is_empty() {
            out.push(format!("{} story is still being written.", cap(if they == "she" { "her" } else if they == "he" { "his" } else { "their" })));
        }
        out
    }

    /// One line to remember the dead by: their name and the two or three
    /// things that mattered most.
    pub fn eulogy(&self, d: &Dwarf) -> String {
        let mut bits: Vec<String> = Vec::new();
        if let Some(deed) = &d.deed {
            bits.push(deed.clone());
        }
        if let Some(sp) = d.spouse.and_then(|id| self.dwarves.iter().find(|o| o.id == id)) {
            bits.push(format!("married {}", first_name(&sp.name)));
        }
        if d.kills > 0 {
            bits.push(format!("slew {}", d.kills));
        }
        let kids = self.dwarves.iter().filter(|o| o.parents.map_or(false, |(m, f)| m == d.id || f == d.id)).count();
        if kids > 0 {
            bits.push(format!("raised {} child{}", kids, if kids == 1 { "" } else { "ren" }));
        }
        if bits.len() < 2 {
            bits.push(format!("a {}", d.profession().to_lowercase()));
        }
        let mut s = format!("{}: {}", d.called(), list(&bits.into_iter().take(3).collect::<Vec<_>>()));
        if let Some(n) = self.annals.deaths.iter().rev().find(|n| n.name == d.name) {
            s.push_str(&format!("; {}", n.cause));
            if let Some(w) = &n.last_words {
                s.push_str(&format!(", saying \u{201c}{}\u{201d}", w));
            }
        }
        s.push('.');
        // Who they left (familytree.rs).
        if let Some(l) = self.left_behind(d) {
            s.push_str(&format!(" {}", l));
        }
        s
    }

    /// Daily, from the overseer's round: it remembers each dwarf who died
    /// since its last round in a line of its notes, and the lines are kept
    /// for the year's chronicle. The round comes early in a step, so a death
    /// later in that step (on the same tick) is caught next time.
    pub(super) fn ov_eulogies(&mut self, o: &mut Overseer) {
        let since = if o.eulogy_from == 0 { self.tick.saturating_sub(DAY) + 1 } else { o.eulogy_from };
        let now = self.tick;
        let dead: Vec<(u32, u64, String)> = self
            .dwarves
            .iter()
            .filter(|d| !d.ghost && !o.eulogized.contains(&d.id) && d.died.map_or(false, |t| t >= since))
            .map(|d| (d.id, d.died.unwrap_or(0), self.eulogy(d)))
            .collect();
        if since != now {
            o.eulogized.clear();
        }
        o.eulogy_from = now;
        o.eulogized.extend(dead.iter().filter(|x| x.1 >= now).map(|x| x.0));
        let lines: Vec<String> = dead.into_iter().map(|x| x.2).collect();
        for l in lines {
            Self::note_ov(o, self.tick, format!("Remembered {}", l));
            self.count("eulogies", 1);
        }
    }

    /// For the year's chronicle: the death scene of each notable dwarf who
    /// died in it (a hero, a noble, a soldier, a parent, a spouse, a killer
    /// of foes, or one who spoke last words), at most four.
    pub(super) fn death_scenes(&self, year_start: u64) -> Vec<String> {
        self.dwarves
            .iter()
            .filter(|d| !d.alive && d.died.map_or(false, |t| t >= year_start && t <= self.tick))
            .filter(|d| {
                d.epithet.is_some()
                    || d.spouse.is_some()
                    || d.kills > 0
                    || d.deed.is_some()
                    || d.noble.is_some()
                    || d.soldier
                    || self.dwarves.iter().any(|o| o.family_of(d.id))
                    || self.annals.deaths.iter().rev().find(|n| n.name == d.name).map_or(false, |n| n.last_words.is_some())
            })
            .take(4)
            .map(|d| self.death_scene(d))
            .collect()
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn list(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_life_is_told_from_its_thoughts_family_wounds_and_death() {
        let mut g = Game::new(5);
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        let bname = g.dwarves[1].name.clone();
        g.dwarves[0].spouse = Some(b);
        g.dwarves[1].spouse = Some(a);
        g.dwarves[0].think(&format!("married {}", bname), -40, SEASON + 5);
        g.dwarves[0].kills = 3;
        g.dwarves[0].med.push((SEASON * 2, "gashed in the head by a goblin".into()));
        g.dwarves[0].body.lost[crate::body::Part::LeftArm.idx()] = true;
        let story = g.life_story(&g.dwarves[0]).join("\n");
        println!("{}", story);
        assert!(story.contains(&format!("married {}", bname)), "{}", story);
        assert!(story.contains("slew 3 foes"));
        assert!(story.contains("gashed in the head by a goblin"));
        assert!(story.contains("lost") && story.contains("left arm"));
        assert!(story.contains(&format!("Summer of {}", g.start_year)), "memories by season");
        assert!(face_scar(&g.dwarves[0]));
        // The living are told in the present: their friends are, their
        // craft is, and an expert is "an expert".
        g.dwarves[0].relations.push((b, 90));
        g.dwarves[0].skills[0] = 1_000_000;
        let story = g.life_story(&g.dwarves[0]).join("\n");
        assert!(story.contains(" closest friend is ") && story.contains(" is a legendary "), "{}", story);
        g.dwarves[0].skills[0] = (0..1_000_000).find(|x| crate::dwarf::skill_rank(*x) == "Expert").unwrap();
        g.dwarves[0].skills.iter_mut().skip(1).for_each(|s| *s = 0);
        assert!(g.life_story(&g.dwarves[0]).join("\n").contains(" is an expert "));
        // Dead: how and when.
        g.tick = SEASON * 3;
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.kill_for_test(&mut d, "was struck down");
        g.dwarves[0] = d;
        let story = g.life_story(&g.dwarves[0]).join("\n");
        assert!(story.contains(" was a "), "past tense for the dead: {}", story);
        assert!(story.lines().last().unwrap().contains("Winter of"), "{}", story);
        let e = g.eulogy(&g.dwarves[0]);
        assert!(e.contains(&format!("married {}", first_name(&bname))) && e.contains("slew 3"), "{}", e);
        assert_eq!(g.death_scenes(0).len(), 1);
    }

    #[test]
    fn looks_are_fixed_and_vary() {
        let g = Game::new(5);
        let all: Vec<Looks> = g.dwarves.iter().map(looks).collect();
        assert_eq!(all, g.dwarves.iter().map(looks).collect::<Vec<_>>());
        assert!(all.iter().any(|l| l.beard != all[0].beard || l.hair != all[0].hair || l.hair_color != all[0].hair_color));
    }

    #[test]
    fn the_overseer_remembers_the_dead() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        g.tick = 3 * DAY;
        let mut d = std::mem::take(&mut g.dwarves[2]);
        g.kill_for_test(&mut d, "was struck down");
        let name = d.name.clone();
        g.dwarves[2] = d;
        let mut o = g.overseer.take().unwrap();
        g.tick += 10;
        g.ov_eulogies(&mut o);
        assert!(o.notes.iter().any(|n| n.1.starts_with("Remembered") && n.1.contains(&name)), "{:?}", o.notes);
        // A death later in the step of a round (on its tick) is remembered
        // at the next round, and nobody twice.
        let round = g.tick;
        let mut d = std::mem::take(&mut g.dwarves[3]);
        g.kill_for_test(&mut d, "was struck down");
        d.died = Some(round);
        let late = d.name.clone();
        g.dwarves[3] = d;
        g.tick += DAY;
        g.ov_eulogies(&mut o);
        g.tick += DAY;
        g.ov_eulogies(&mut o);
        let count = |who: &str| o.notes.iter().filter(|n| n.1.starts_with("Remembered") && n.1.contains(who)).count();
        assert_eq!((count(&name), count(&late)), (1, 1), "{:?}", o.notes);
    }

    #[test]
    fn an_engraver_is_an_engraver() {
        let mut g = Game::new(5);
        g.dwarves[0].age = None;
        let p = g.dwarves[0].profession().to_lowercase();
        let first = g.life_story(&g.dwarves[0])[0].clone();
        let a = if p.starts_with(|c| "aeiou".contains(c)) { " is an " } else { " is a " };
        assert!(first.contains(&format!("{}{}", a, p)), "{}", first);
        g.dwarves[0].age = Some(80);
        assert!(g.life_story(&g.dwarves[0])[0].contains(" is an 80-year-old "));
    }
}
