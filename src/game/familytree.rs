//! A dwarf's family tree (O06, UI-15) and how the dead are told (UI-08).
//!
//! The tree is worked out by scanning the dwarves (a fort holds a couple of
//! hundred at most): grandparents, parents, the dwarf with their spouses
//! (past ones from `Game.memory`'s weddings) and siblings, children,
//! grandchildren, their animals, and the friends and grudges outside the
//! family. Each relative carries a mark for how they and the dwarf feel
//! about each other, and the dead say where they lie.
//!
//! A death scene is the one sentence the fort tells of a death: who, how,
//! where (from the death's memory), when, in which siege, the last words
//! (O05) and who was left behind. The chronicle, the Life page and the tree
//! all tell it. Nothing here draws on the game's random numbers.

use super::memory::MemoryKind;
use super::*;

/// How a relative and the dwarf feel about each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    None,
    /// One of them thinks the world of the other, and neither holds a grudge.
    Friend,
    /// One of them holds a grudge.
    Grudge,
}

/// Regard at or above this is a friendship.
pub const FRIEND: i32 = 50;
/// Regard at or below this is a grudge.
pub const GRUDGE: i32 = -40;

/// Someone on the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    Dwarf(u32),
    Beast(u32),
}

/// One relative: who, what they are to the dwarf ("mother", "late wife"),
/// and the mark between them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kin {
    pub who: Who,
    pub rel: String,
    pub mark: Mark,
}

/// A row of the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generation {
    pub title: &'static str,
    pub kin: Vec<Kin>,
}

fn word(female: Option<bool>, she: &'static str, he: &'static str, they: &'static str) -> &'static str {
    match female {
        Some(true) => she,
        Some(false) => he,
        None => they,
    }
}

fn first(name: &str) -> &str {
    name.split(' ').next().unwrap_or(name)
}

impl Game {
    /// A dwarf of the fort by id, at home or away on the expedition.
    pub fn kin_dwarf(&self, id: u32) -> Option<&Dwarf> {
        self.dwarves.iter().find(|d| d.id == id).or_else(|| self.expedition.as_ref().and_then(|ex| ex.members.iter().find(|d| d.id == id)))
    }

    fn all_dwarves(&self) -> impl Iterator<Item = &Dwarf> {
        self.dwarves.iter().chain(self.expedition.iter().flat_map(|ex| ex.members.iter()))
    }

    /// The mark between two dwarves: a grudge either way wins over a
    /// friendship.
    pub fn bond(&self, a: u32, b: u32) -> Mark {
        let (Some(x), Some(y)) = (self.kin_dwarf(a), self.kin_dwarf(b)) else { return Mark::None };
        let (ab, ba) = (x.regard(b), y.regard(a));
        if ab <= GRUDGE || ba <= GRUDGE {
            Mark::Grudge
        } else if ab >= FRIEND || ba >= FRIEND {
            Mark::Friend
        } else {
            Mark::None
        }
    }

    fn parents_of(&self, id: u32) -> Vec<u32> {
        self.kin_dwarf(id).and_then(|d| d.parents).map_or(Vec::new(), |(m, f)| [m, f].into_iter().filter(|&p| p != id && self.kin_dwarf(p).is_some()).collect())
    }

    fn children_of(&self, id: u32) -> Vec<u32> {
        self.all_dwarves().filter(|d| d.family_of(id)).map(|d| d.id).collect()
    }

    /// Everyone the dwarf has been married to, the present spouse first:
    /// the spouse, then the weddings the fort remembers.
    pub fn spouses_of(&self, id: u32) -> Vec<u32> {
        let mut v: Vec<u32> = self.kin_dwarf(id).and_then(|d| d.spouse).into_iter().collect();
        for m in self.memory.events.iter().filter(|m| m.kind == MemoryKind::Married && m.who.contains(&id)) {
            for &o in &m.who {
                if o != id && !v.contains(&o) && self.kin_dwarf(o).is_some() {
                    v.push(o);
                }
            }
        }
        v.retain(|&o| self.kin_dwarf(o).is_some());
        v
    }

    /// The dwarf's family tree, a row per generation (only the rows with
    /// someone in them, but always the dwarf's own).
    pub fn family_tree(&self, id: u32) -> Vec<Generation> {
        let Some(me) = self.kin_dwarf(id) else { return Vec::new() };
        let mut seen: Vec<u32> = vec![id];
        let mut rows = Vec::new();
        let row = |title: &'static str, ids: Vec<(u32, String)>, seen: &mut Vec<u32>, rows: &mut Vec<Generation>| {
            let mut kin = Vec::new();
            for (o, rel) in ids {
                if !seen.contains(&o) {
                    seen.push(o);
                    kin.push(Kin { who: Who::Dwarf(o), rel, mark: self.bond(id, o) });
                }
            }
            if !kin.is_empty() {
                rows.push(Generation { title, kin });
            }
        };
        let sex = |o: u32| self.kin_dwarf(o).and_then(|d| d.female);
        let parents = self.parents_of(id);
        let grand: Vec<(u32, String)> = parents.iter().flat_map(|&p| self.parents_of(p)).map(|g| (g, word(sex(g), "grandmother", "grandfather", "grandparent").to_string())).collect();
        row("Grandparents", grand, &mut seen, &mut rows);
        row("Parents", parents.iter().map(|&p| (p, word(sex(p), "mother", "father", "parent").to_string())).collect(), &mut seen, &mut rows);
        // The dwarf's own generation: them, their spouses, their siblings.
        let mut own = vec![Kin { who: Who::Dwarf(id), rel: "themself".into(), mark: Mark::None }];
        let spouses = self.spouses_of(id);
        for (i, &s) in spouses.iter().enumerate() {
            let dead = self.kin_dwarf(s).map_or(false, |d| !d.alive);
            let w = word(sex(s), "wife", "husband", "spouse");
            let rel = match (i, dead) {
                (0, false) => w.to_string(),
                (_, true) => format!("late {}", w),
                _ => format!("former {}", w),
            };
            if !seen.contains(&s) {
                seen.push(s);
                own.push(Kin { who: Who::Dwarf(s), rel, mark: self.bond(id, s) });
            }
        }
        let mut sibs: Vec<u32> = if parents.is_empty() { Vec::new() } else { self.all_dwarves().filter(|d| d.id != id && d.parents.map_or(false, |(m, f)| parents.contains(&m) || parents.contains(&f))).map(|d| d.id).collect() };
        sibs.sort_by_key(|&s| std::cmp::Reverse(self.kin_dwarf(s).and_then(|d| d.age).unwrap_or(0)));
        for s in sibs {
            if !seen.contains(&s) {
                seen.push(s);
                own.push(Kin { who: Who::Dwarf(s), rel: word(sex(s), "sister", "brother", "sibling").into(), mark: self.bond(id, s) });
            }
        }
        rows.push(Generation { title: "Spouses and siblings", kin: own });
        let kids = self.children_of(id);
        row("Children", kids.iter().map(|&c| (c, word(sex(c), "daughter", "son", "child").to_string())).collect(), &mut seen, &mut rows);
        let grandkids: Vec<(u32, String)> = kids.iter().flat_map(|&c| self.children_of(c)).map(|c| (c, word(sex(c), "granddaughter", "grandson", "grandchild").to_string())).collect();
        row("Grandchildren", grandkids, &mut seen, &mut rows);
        // Their animals.
        let mut pets: Vec<&crate::creature::Creature> = self.creatures.values().filter(|c| c.alive && c.tame && (c.pet_of == Some(id) || c.owner == Some(id))).collect();
        pets.sort_by_key(|c| c.id);
        if !pets.is_empty() {
            rows.push(Generation {
                title: "Animals",
                kin: pets.into_iter().map(|c| Kin { who: Who::Beast(c.id), rel: if c.owner == Some(id) { "war animal".into() } else { "pet".into() }, mark: Mark::None }).collect(),
            });
        }
        // Friends and grudges outside the family, the strongest first.
        let mut others: Vec<(i32, u32)> = me
            .relations
            .iter()
            .filter(|(o, _)| !seen.contains(o) && self.kin_dwarf(*o).is_some())
            .map(|&(o, r)| (r, o))
            .filter(|&(_, o)| self.bond(id, o) != Mark::None)
            .collect();
        others.sort_by_key(|&(r, o)| (std::cmp::Reverse(r.abs()), o));
        let others: Vec<Kin> = others
            .into_iter()
            .take(6)
            .map(|(_, o)| {
                let mark = self.bond(id, o);
                Kin { who: Who::Dwarf(o), rel: if mark == Mark::Grudge { "grudge".into() } else { "friend".into() }, mark }
            })
            .collect();
        if !others.is_empty() {
            rows.push(Generation { title: "Friends and grudges", kin: others });
        }
        rows
    }

    /// The kin of `id` who already lie in coffins: where, and what they
    /// were to them ("her husband Kib").
    fn kin_graves(&self, id: u32) -> Vec<(Pos, String)> {
        let Some(d) = self.kin_dwarf(id) else { return Vec::new() };
        let her = word(d.female, "her", "his", "their");
        let sex = |o: u32| self.kin_dwarf(o).and_then(|x| x.female);
        let mut kin: Vec<(u32, &str)> = self.spouses_of(id).into_iter().map(|s| (s, word(sex(s), "wife", "husband", "spouse"))).collect();
        kin.extend(self.parents_of(id).into_iter().map(|p| (p, word(sex(p), "mother", "father", "parent"))));
        kin.extend(self.children_of(id).into_iter().map(|c| (c, word(sex(c), "daughter", "son", "child"))));
        let mut v = Vec::new();
        for (k, rel) in kin {
            let Some(name) = self.kin_dwarf(k).map(|x| first(&x.name).to_string()) else { continue };
            if let Some(b) = self.buildings.values().find(|b| b.kind == BuildingKind::Coffin && b.occupant == Some(k)) {
                v.push((b.pos, format!("{} {} {}", her, rel, name)));
            }
        }
        v
    }

    /// A free coffin beside the graves of `dead`'s kin, if any of them lie
    /// in coffins: in a tomb first, then the nearest. The haulers bury
    /// families together.
    pub(super) fn coffin_beside_kin(&self, dead: u32) -> Option<u32> {
        let graves = self.kin_graves(dead);
        if graves.is_empty() {
            return None;
        }
        let dist = |p: Pos| graves.iter().filter(|g| g.0.z == p.z).map(|g| (g.0.x - p.x).abs().max((g.0.y - p.y).abs())).min().unwrap_or(i32::MAX);
        self.buildings
            .values()
            .filter(|b| b.kind == BuildingKind::Coffin && b.built && b.occupant.is_none() && b.worker.is_none())
            .min_by_key(|b| (self.zone_kind_at(b.pos) != Some(ZoneKind::Tomb), dist(b.pos), b.id))
            .filter(|b| dist(b.pos) < i32::MAX)
            .map(|b| b.id)
    }

    /// Whose grave a coffin at `p` lies beside: "her husband Kib".
    pub fn beside_kin(&self, dead: u32, p: Pos) -> Option<String> {
        self.kin_graves(dead).into_iter().find(|g| g.0 != p && g.0.z == p.z && (g.0.x - p.x).abs() <= 2 && (g.0.y - p.y).abs() <= 2).map(|g| g.1)
    }

    /// Where a dead dwarf lies: "lies in a coffin in the Hall of Tombs",
    /// "is remembered on a slab", "lies unburied", "walks as a ghost".
    /// None for the living.
    pub fn resting_place(&self, d: &Dwarf) -> Option<String> {
        if d.alive {
            return None;
        }
        if d.ghost {
            return Some("walks the halls as a ghost".into());
        }
        let b = self.buildings.values().find(|b| b.occupant == Some(d.id) && matches!(b.kind, BuildingKind::Coffin | BuildingKind::Slab));
        let place = |p: Pos| {
            self.zone_at(p)
                .and_then(|z| z.name.clone())
                .or_else(|| self.place_name(p))
                .or_else(|| self.place_at(p).map(|pl| pl.name.clone()))
                .or_else(|| (self.zone_kind_at(p) == Some(ZoneKind::Tomb)).then(|| "a tomb".to_string()))
        };
        Some(match b {
            Some(b) if b.kind == BuildingKind::Slab => match place(b.pos) {
                Some(n) => format!("is remembered on a slab in {}", n),
                None => "is remembered on a memorial slab".into(),
            },
            Some(b) => {
                let mut s = match place(b.pos) {
                    Some(n) => format!("lies in a coffin in {}", n),
                    None => "lies in a coffin".into(),
                };
                if let Some(k) = self.beside_kin(d.id, b.pos) {
                    s.push_str(&format!(", beside {}", k));
                }
                s
            }
            None if d.rested => "was laid to rest".into(),
            None => "lies unburied".into(),
        })
    }

    /// Who a death left behind: "She left Kib a widower and two children."
    pub fn left_behind(&self, d: &Dwarf) -> Option<String> {
        let mut left = Vec::new();
        if let Some(sp) = d.spouse.and_then(|id| self.kin_dwarf(id)).filter(|sp| sp.alive && sp.spouse == Some(d.id)) {
            left.push(format!("{} {}", first(&sp.name), word(sp.female, "a widow", "a widower", "alone")));
        }
        let kids = self.all_dwarves().filter(|o| o.alive && o.family_of(d.id)).count();
        match kids {
            0 => {}
            1 => left.push("a child".into()),
            n => left.push(format!("{} children", n)),
        }
        (!left.is_empty()).then(|| format!("{} left {}.", word(d.female, "She", "He", "They"), left.join(" and ")))
    }

    /// The one sentence the fort tells of a death: "Urist Copperhand, an
    /// 80-year-old mason, was struck down by a goblin in the Great Dining
    /// Hall in the Spring of 203, in the siege of the Black Spears. Her
    /// last words were “…”. She left Kib a widower, and two children."
    pub fn death_scene(&self, d: &Dwarf) -> String {
        let note = self.annals.deaths.iter().rev().find(|n| n.name == d.name);
        let prof = d.profession().to_lowercase();
        let what = match d.age {
            Some(a) => format!("{}-year-old {}", a, prof),
            None => prof,
        };
        let a = if what.starts_with(|c: char| "aeiou".contains(c)) || what.starts_with("8") || what.starts_with("11-") || what.starts_with("18-") { "an" } else { "a" };
        // Where and how, from the death's memory and its note.
        let mem = self.memory.events.iter().rev().find(|m| m.kind == MemoryKind::Died && m.who.first() == Some(&d.id));
        let told = || {
            let t = mem?.text.strip_prefix(&d.called())?.trim();
            let t = t.split(", saying").next()?.trim_end_matches('.');
            let t = match t.strip_prefix("has been ") {
                Some(r) => format!("was {}", r),
                None => t.strip_prefix("has ").unwrap_or(t).to_string(),
            };
            (!t.is_empty()).then_some(t)
        };
        let how = match note.map(|n| (n.killer.as_deref(), n.cause.as_str())) {
            Some((Some(k), "struck down")) => format!("was struck down by a {}", k),
            Some((Some(k), "bled to death")) => format!("bled to death from {} wounds", k),
            Some((_, "struck down")) => "was struck down".into(),
            Some((_, "bled to death")) => "bled to death".into(),
            Some((_, "thirst")) => "died of thirst".into(),
            Some((_, "starvation")) => "starved to death".into(),
            Some((_, "burned")) => "burned to death".into(),
            Some((_, "crushed")) => "was crushed".into(),
            Some((_, "old age")) => "died of old age".into(),
            Some((_, "drowned")) => "drowned".into(),
            Some((_, "fell on an expedition")) => "fell on an expedition".into(),
            _ => told().unwrap_or_else(|| "died".into()),
        };
        let at = mem.and_then(|m| m.at).and_then(|p| {
            self.place_at(p).map(|pl| format!(" at {}", pl.name)).or_else(|| self.zone_at(p).and_then(|z| z.name.clone()).or_else(|| self.place_name(p)).map(|n| format!(" in {}", n)))
        });
        let when = d.died.or(mem.map(|m| m.tick)).map_or(String::new(), |t| format!(" in the {} of {}", SEASONS[((t % YEAR) / SEASON) as usize], self.start_year as u64 + t / YEAR));
        let siege = note.and_then(|n| n.siege).and_then(|i| self.annals.sieges.get(i)).map_or(String::new(), |s| match &s.name {
            Some(name) => format!(", in {}", name),
            None => format!(", in the siege of the {}", s.foe),
        });
        let mut s = format!("{}, {} {}, {}{}{}{}.", d.called(), a, what, how, at.unwrap_or_default(), when, siege);
        let her = word(d.female, "Her", "His", "Their");
        if let Some(w) = note.and_then(|n| n.last_words.as_ref()) {
            s.push_str(&format!(" {} last words were \u{201c}{}\u{201d}", her, w));
            if !s.ends_with('.') && !s.ends_with('!') && !s.ends_with('?') && !s.ends_with('\u{201d}') {
                s.push('.');
            }
        }
        if let Some(l) = self.left_behind(d) {
            s.push_str(&format!(" {}", l));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two parents, their son and daughter, the son's wife and child.
    fn family() -> (Game, [u32; 6]) {
        let mut g = Game::new(5);
        let ids: Vec<u32> = g.dwarves.iter().take(6).map(|d| d.id).collect();
        let [ma, pa, son, girl, wife, baby] = [ids[0], ids[1], ids[2], ids[3], ids[4], ids[5]];
        let set = |g: &mut Game, id: u32, female: bool, parents: Option<(u32, u32)>| {
            let d = g.dwarves.iter_mut().find(|d| d.id == id).unwrap();
            d.female = Some(female);
            d.parents = parents;
            d.relations.clear();
        };
        set(&mut g, ma, true, None);
        set(&mut g, pa, false, None);
        set(&mut g, son, false, Some((ma, pa)));
        set(&mut g, girl, true, Some((ma, pa)));
        set(&mut g, wife, true, None);
        set(&mut g, baby, true, Some((wife, son)));
        for (a, b) in [(ma, pa), (son, wife)] {
            g.dwarves.iter_mut().find(|d| d.id == a).unwrap().spouse = Some(b);
            g.dwarves.iter_mut().find(|d| d.id == b).unwrap().spouse = Some(a);
        }
        (g, [ma, pa, son, girl, wife, baby])
    }

    #[test]
    fn the_tree_has_every_generation_and_the_marks() {
        let (mut g, [ma, pa, son, girl, wife, baby]) = family();
        g.dwarves.iter_mut().find(|d| d.id == son).unwrap().adjust_regard(girl, -60);
        g.dwarves.iter_mut().find(|d| d.id == wife).unwrap().adjust_regard(son, 80);
        let tree = g.family_tree(son);
        let titles: Vec<&str> = tree.iter().map(|r| r.title).collect();
        assert_eq!(titles, vec!["Parents", "Spouses and siblings", "Children"]);
        let rel = |id: u32| tree.iter().flat_map(|r| r.kin.iter()).find(|k| k.who == Who::Dwarf(id)).unwrap_or_else(|| panic!("{} not on {:?}", id, tree)).clone();
        assert_eq!(rel(ma).rel, "mother");
        assert_eq!(rel(pa).rel, "father");
        assert_eq!(rel(wife).rel, "wife");
        assert_eq!(rel(girl).rel, "sister");
        assert_eq!(rel(baby).rel, "daughter");
        assert_eq!(rel(girl).mark, Mark::Grudge);
        assert_eq!(rel(wife).mark, Mark::Friend);
        // From the baby: grandparents, an aunt is not drawn, but her
        // father's family is.
        let up = g.family_tree(baby);
        assert_eq!(up[0].title, "Grandparents");
        assert!(up[0].kin.iter().any(|k| k.who == Who::Dwarf(ma) && k.rel == "grandmother"));
        // From the grandmother: a grandchild.
        assert!(g.family_tree(ma).iter().any(|r| r.title == "Grandchildren" && r.kin[0].who == Who::Dwarf(baby)));
    }

    #[test]
    fn the_dead_say_where_they_lie_and_widows_remarry() {
        let (mut g, [_, _, son, _, wife, _]) = family();
        let i = g.dwarves.iter().position(|d| d.id == son).unwrap();
        let mut d = std::mem::take(&mut g.dwarves[i]);
        g.kill_for_test(&mut d, "was struck down");
        g.dwarves[i] = d;
        assert_eq!(g.resting_place(&g.dwarves[i]).as_deref(), Some("lies unburied"));
        let e = g.embark;
        let p = (2..20).map(|r| e.offset(r, 1, 0)).find(|&p| g.can_place(BuildingKind::Coffin, p)).unwrap();
        assert!(g.place_building(BuildingKind::Coffin, p));
        let coffin = g.world.t(p).building;
        g.buildings.get_mut(&coffin).unwrap().occupant = Some(son);
        g.dwarves[i].rested = true;
        assert!(g.resting_place(&g.dwarves[i]).unwrap().starts_with("lies in a coffin"));
        assert!(g.resting_place(&g.dwarves[0].clone()).is_none() || !g.dwarves[0].alive);
        // The widow remarries: the first husband is still on her tree, late.
        let other = g.dwarves.iter().find(|d| d.alive && ![son, wife].contains(&d.id) && d.parents.is_none() && d.spouse.is_none()).unwrap().id;
        g.record(MemoryKind::Married, &[son, wife], None, "wed".into(), 4);
        g.dwarves.iter_mut().find(|d| d.id == wife).unwrap().spouse = Some(other);
        g.dwarves.iter_mut().find(|d| d.id == other).unwrap().female = Some(false);
        let tree = g.family_tree(wife);
        let own = tree.iter().find(|r| r.title == "Spouses and siblings").unwrap();
        assert!(own.kin.iter().any(|k| k.who == Who::Dwarf(other) && k.rel.ends_with("band")), "{:?}", own);
        assert!(own.kin.iter().any(|k| k.who == Who::Dwarf(son) && k.rel == "late husband"), "{:?}", own);
    }

    #[test]
    fn a_death_scene_tells_where_when_the_last_words_and_who_was_left() {
        let (mut g, [_, _, son, _, wife, _]) = family();
        g.tick = SEASON + 50;
        let i = g.dwarves.iter().position(|d| d.id == son).unwrap();
        let mut d = std::mem::take(&mut g.dwarves[i]);
        g.kill_for_test(&mut d, "was struck down");
        g.dwarves[i] = d;
        if let Some(n) = g.annals.deaths.last_mut() {
            n.last_words = Some("Tell my daughter I was brave".into());
        }
        let at = g.dwarves[i].pos;
        g.stories.places.push(crate::game::stories::Place { name: "Kolvar's Stand".into(), at, r: 3, deed: String::new(), year: 0, kind: Default::default(), b: None, weight: 5, by_player: false });
        let s = g.death_scene(&g.dwarves[i]);
        println!("{}", s);
        let wname = g.dwarves.iter().find(|d| d.id == wife).unwrap().name.clone();
        assert!(s.starts_with(&g.dwarves[i].called()), "{}", s);
        assert!(s.contains("at Kolvar's Stand"), "{}", s);
        assert!(s.contains(&format!("Summer of {}", g.start_year)), "{}", s);
        assert!(s.contains("His last words were \u{201c}Tell my daughter I was brave\u{201d}"), "{}", s);
        assert!(s.contains(&format!("left {} a widow and a child", first(&wname))), "{}", s);
    }

    #[test]
    fn families_are_buried_together() {
        let (mut g, [_, _, son, _, wife, _]) = family();
        let e = g.embark;
        let spots: Vec<Pos> = (2..30).map(|r| e.offset(r, 1, 0)).filter(|&p| g.can_place(BuildingKind::Coffin, p)).collect();
        let (near_him, far, his) = (spots[spots.len() - 2], spots[0], spots[spots.len() - 1]);
        let coffin = |g: &mut Game, p: Pos| {
            assert!(g.place_building(BuildingKind::Coffin, p));
            let id = g.world.t(p).building;
            g.buildings.get_mut(&id).unwrap().built = true;
            id
        };
        let a = coffin(&mut g, far);
        let b = coffin(&mut g, near_him);
        let c = coffin(&mut g, his);
        assert!(a < b, "the far coffin would be the first pick");
        assert_eq!(g.coffin_beside_kin(wife), None, "nobody of hers lies in a coffin yet");
        g.buildings.get_mut(&c).unwrap().occupant = Some(son);
        assert_eq!(g.coffin_beside_kin(wife), Some(b), "beside her husband");
        assert!(g.beside_kin(wife, near_him).unwrap().starts_with("her husband "));
    }
}
