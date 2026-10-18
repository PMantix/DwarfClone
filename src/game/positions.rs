//! Appointed positions: the fortress's administrators, each with a duty.

use super::*;
use crate::dwarf::Noble;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    Manager,
    Bookkeeper,
    Broker,
    ChiefMedic,
    Commander,
    Sheriff,
}

impl Position {
    pub const ALL: [Position; 6] =
        [Position::Manager, Position::Bookkeeper, Position::Broker, Position::ChiefMedic, Position::Commander, Position::Sheriff];

    pub fn title(self) -> &'static str {
        match self {
            Position::Manager => "Manager",
            Position::Bookkeeper => "Bookkeeper",
            Position::Broker => "Broker",
            Position::ChiefMedic => "Chief medical dwarf",
            Position::Commander => "Militia commander",
            Position::Sheriff => "Sheriff",
        }
    }

    /// What having one does, for the nobles screen.
    pub fn duty(self) -> &'static str {
        match self {
            Position::Manager => "work orders (j) are carried out",
            Position::Bookkeeper => "stocks are counted often, not daily",
            Position::Broker => "better prices from traders",
            Position::ChiefMedic => "doctors treat a quarter faster",
            Position::Commander => "soldiers learn a quarter faster",
            Position::Sheriff => "crimes on the record are punished",
        }
    }

    /// How well a dwarf suits this post: a score (higher is better) and the
    /// qualities behind it, for the appointment list.
    pub fn suitability(self, d: &Dwarf) -> (i32, Vec<String>) {
        let t = &d.personality;
        let fight = dwarf::skill_level(d.fight_xp) as i32;
        let medic = d.skill(Labor::Medicine) as i32;
        let mut notes: Vec<String> = Vec::new();
        fn word(notes: &mut Vec<String>, x: i32, hi: &str, lo: &str) {
            if x >= 20 {
                notes.push(hi.to_string());
            } else if x <= -20 {
                notes.push(lo.to_string());
            }
        }
        let score = match self {
            Position::Manager => {
                word(&mut notes, t.diligence, "diligent", "lazy");
                word(&mut notes, t.sociability, "sociable", "aloof");
                t.diligence + t.sociability / 2
            }
            Position::Bookkeeper => {
                word(&mut notes, t.diligence, "diligent", "lazy");
                word(&mut notes, -t.anxiety, "calm", "anxious");
                t.diligence - t.anxiety / 2
            }
            Position::Broker => {
                word(&mut notes, t.sociability, "sociable", "aloof");
                word(&mut notes, t.temper, "hot-tempered", "even-tempered");
                t.sociability - t.temper / 3
            }
            Position::ChiefMedic => {
                notes.push(format!("{} doctor", dwarf::skill_rank(d.skills[Labor::Medicine.idx()])));
                word(&mut notes, t.diligence, "diligent", "lazy");
                medic * 15 + t.diligence / 3 - 10
            }
            Position::Commander => {
                notes.push(format!("{} fighter", dwarf::skill_rank(d.fight_xp)));
                word(&mut notes, t.sociability, "sociable", "aloof");
                fight * 15 + t.sociability / 3 - 10
            }
            Position::Sheriff => {
                notes.push(format!("{} fighter", dwarf::skill_rank(d.fight_xp)));
                word(&mut notes, t.temper, "hot-tempered", "even-tempered");
                fight * 15 + t.diligence / 3 - t.temper / 3 - 10
            }
        };
        (score, notes)
    }
}

/// How well a post's holder suits it, which sets how well they do it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Poor,
    Fair,
    Good,
}

impl Fit {
    pub const ALL: [Fit; 3] = [Fit::Good, Fit::Fair, Fit::Poor];

    pub fn of(score: i32) -> Fit {
        if score >= 25 {
            Fit::Good
        } else if score >= 0 {
            Fit::Fair
        } else {
            Fit::Poor
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Fit::Good => "good fit",
            Fit::Fair => "fair fit",
            Fit::Poor => "poor fit",
        }
    }

    pub fn color(self) -> (u8, u8, u8) {
        match self {
            Fit::Good => (110, 220, 110),
            Fit::Fair => (230, 200, 90),
            Fit::Poor => (230, 110, 90),
        }
    }
}

impl Position {
    /// What a holder of each fit brings to the post.
    pub fn effect(self, f: Fit) -> &'static str {
        use Fit::*;
        match (self, f) {
            (Position::Manager, Good) => "orders seen to twice a day",
            (Position::Manager, Fair) => "orders seen to once a day",
            (Position::Manager, Poor) => "one job per order at a time",
            (Position::Bookkeeper, Good) => "stock counted 8 times a day",
            (Position::Bookkeeper, Fair) => "stock counted 4 times a day",
            (Position::Bookkeeper, Poor) => "stock counted twice a day",
            (Position::Broker, Good) => "markup haggled to almost nothing",
            (Position::Broker, Fair) => "markup roughly halved",
            (Position::Broker, Poor) => "markup barely cut",
            (Position::ChiefMedic, Good) => "treatment 40% faster",
            (Position::ChiefMedic, Fair) => "treatment 25% faster",
            (Position::ChiefMedic, Poor) => "treatment 10% faster",
            (Position::Commander, Good) => "training 50% faster",
            (Position::Commander, Fair) => "training 25% faster",
            (Position::Commander, Poor) => "training 12% faster",
            (Position::Sheriff, Good) => "culprits never get away",
            (Position::Sheriff, Fair) => "culprits now and then get away",
            (Position::Sheriff, Poor) => "culprits often get away",
        }
    }
}

/// A titled noble wants a piece of furniture of a certain material in
/// their bedroom or office.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Demand {
    pub noble: u32,
    pub kind: BuildingKind,
    pub mat: Mat,
    pub deadline: u64,
}

impl Demand {
    pub fn describe(&self) -> String {
        format!("{} {}", self.mat.adj(), self.kind.name().to_lowercase())
    }
}

/// (title, wealth, population) needed for each title, lowest first.
pub const TITLES: [(Noble, u32, usize); 3] = [(Noble::Baron, 25_000, 20), (Noble::Count, 60_000, 30), (Noble::Duke, 120_000, 40)];

impl Game {
    /// Who holds a position, if they're alive and in their right mind.
    pub fn holder(&self, p: Position) -> Option<&Dwarf> {
        let id = self.positions.iter().find(|(q, _)| *q == p).map(|(_, id)| *id)?;
        self.dwarves.iter().find(|d| d.id == id && d.alive && !d.insane)
    }

    /// Whether a post is filled. Unlike `holder`, this also counts a holder
    /// who is out of the dwarf list for their own turn.
    pub fn has_post(&self, p: Position) -> bool {
        let Some(id) = self.positions.iter().find(|(q, _)| *q == p).map(|(_, id)| *id) else { return false };
        match self.dwarves.iter().find(|d| d.id == id) {
            Some(d) => d.alive && !d.insane,
            None => true,
        }
    }

    /// How well a dwarf would suit a post here: their own qualities, and an
    /// office to work from.
    pub fn post_suitability(&self, p: Position, d: &Dwarf) -> (i32, Vec<String>) {
        let (mut score, mut notes) = p.suitability(d);
        if self.office_of(d.id).is_some() {
            score += 15;
            notes.push("has an office".into());
        }
        (score, notes)
    }

    /// How well the post's holder suits it, if it's filled. A holder out of
    /// the dwarf list for their own turn is judged from the last count.
    pub fn post_fit(&self, p: Position) -> Option<Fit> {
        let id = self.positions.iter().find(|(q, _)| *q == p).map(|(_, id)| *id)?;
        match self.dwarves.iter().find(|d| d.id == id) {
            Some(d) if d.alive && !d.insane => Some(Fit::of(self.post_suitability(p, d).0)),
            Some(_) => None,
            None => Some(self.fit_cache.iter().find(|(q, _)| *q == p).map_or(Fit::Fair, |f| f.1)),
        }
    }

    /// Remember each holder's fit, for when they're mid-turn.
    pub(super) fn count_fits(&mut self) {
        self.fit_cache = Position::ALL.iter().filter_map(|&p| self.post_fit(p).map(|f| (p, f))).collect();
    }

    pub fn position_of(&self, dwarf: u32) -> Option<Position> {
        self.positions.iter().find(|(_, id)| *id == dwarf).map(|(p, _)| *p)
    }

    /// Give a position to a dwarf (taking any other position off them), or
    /// leave it vacant.
    pub fn appoint(&mut self, p: Position, dwarf: Option<u32>) {
        self.positions.retain(|(q, id)| *q != p && Some(*id) != dwarf);
        if let Some(id) = dwarf {
            self.positions.push((p, id));
        }
        self.count_fits();
    }

    /// The dwarves who could take a post, best suited first.
    pub fn candidates(&self, p: Position) -> Vec<u32> {
        let mut v: Vec<(i32, u32)> = self
            .dwarves
            .iter()
            .filter(|d| d.alive && !d.insane && !d.is_child())
            .map(|d| (self.post_suitability(p, d).0, d.id))
            .collect();
        v.sort_by_key(|&(s, id)| (-s, id));
        v.into_iter().map(|(_, id)| id).collect()
    }

    /// Appoint `dwarf` (or nobody) and say what changed.
    pub fn appoint_msg(&mut self, p: Position, dwarf: Option<u32>) -> String {
        let before = dwarf.and_then(|id| self.position_of(id)).filter(|&q| q != p);
        self.appoint(p, dwarf);
        match dwarf.and_then(|id| self.dwarves.iter().find(|d| d.id == id)) {
            Some(d) => match before {
                Some(q) => format!("{} is now {} (and no longer {}).", d.name, p.title().to_lowercase(), q.title().to_lowercase()),
                None => format!("{} is now {}.", d.name, p.title().to_lowercase()),
            },
            None => format!("The post of {} is vacant.", p.title().to_lowercase()),
        }
    }

    /// Hand a position to the next adult in line, or leave it vacant after
    /// the last. Returns a message. (The nobles screen picks from a list now.)
    #[cfg(test)]
    pub fn cycle_position(&mut self, p: Position) -> String {
        let cands: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && !d.insane && !d.is_child()).map(|d| d.id).collect();
        let cur = self.positions.iter().find(|(q, _)| *q == p).map(|(_, id)| *id);
        let next = match cur.and_then(|c| cands.iter().position(|&x| x == c)) {
            None => cands.first().copied(),
            Some(i) => cands.get(i + 1).copied(),
        };
        self.appoint(p, next);
        match next.and_then(|id| self.dwarves.iter().find(|d| d.id == id)) {
            Some(d) => format!("{} is now {}.", d.name, p.title().to_lowercase()),
            None => format!("The post of {} is vacant.", p.title().to_lowercase()),
        }
    }
}

impl Game {
    /// The highest title the fortress's wealth and size have earned.
    pub fn earned_title(&self) -> Option<Noble> {
        let wealth = self.wealth();
        let pop = self.alive_count();
        TITLES.iter().rev().find(|(_, w, p)| wealth >= *w && pop >= *p).map(|t| t.0)
    }

    /// Daily: the civilization raises a landholder as the fortress grows rich,
    /// and titled nobles make (and judge) demands for their rooms.
    pub(super) fn titles_daily(&mut self) {
        let rank = |n: Noble| TITLES.iter().position(|t| t.0 == n);
        if let Some(earned) = self.earned_title() {
            let holder = self.dwarves.iter().position(|d| d.alive && d.noble.map_or(false, |n| rank(n).is_some()));
            let current = holder.and_then(|i| self.dwarves[i].noble).and_then(rank);
            if current.map_or(true, |c| c < rank(earned).unwrap()) {
                let pick = holder.or_else(|| {
                    (0..self.dwarves.len())
                        .filter(|&i| {
                            let d = &self.dwarves[i];
                            d.alive && !d.insane && !d.is_child() && d.noble.is_none()
                        })
                        .max_by_key(|&i| self.dwarves[i].personality.sociability)
                });
                if let Some(i) = pick {
                    self.dwarves[i].noble = Some(earned);
                    let name = self.dwarves[i].name.clone();
                    let fort = self.fortress_name.clone();
                    let civ = self.civ.map_or("the realm".to_string(), |c| format!("the {}", self.overworld.civ(c).name));
                    let title = earned.title().to_lowercase();
                    self.msg(&format!("{} has made {} {} of {}!", cap(&civ), name, title, fort), (255, 215, 60));
                    self.legend(format!("{} was made {} of {}.", name, title, fort));
                    let at = self.dwarves[i].pos;
                    self.spotlight("A new noble", &format!("{} has made {} {} of {}!", cap(&civ), name, title, fort), Some(at), (255, 215, 60));
                }
            }
        }
        self.judge_demands();
        // Each season, a titled noble without a demand may make one.
        if self.tick % SEASON < DAY {
            let titled: Vec<u32> = self
                .dwarves
                .iter()
                .filter(|d| d.alive && !d.insane && d.noble.map_or(false, |n| rank(n).is_some()))
                .map(|d| d.id)
                .filter(|id| !self.demands.iter().any(|m| m.noble == *id))
                .collect();
            for id in titled {
                if self.rng.chance(0.6) {
                    self.make_demand(id);
                }
            }
        }
    }

    fn make_demand(&mut self, noble: u32) {
        let kind = *self.rng.pick(&[BuildingKind::Chair, BuildingKind::Table, BuildingKind::Door, BuildingKind::Coffin]);
        // Something the fortress could make: wood, or a stone it has.
        let mut mats: Vec<Mat> = vec![Mat::Wood];
        for it in self.items.values().filter(|it| it.kind == ItemKind::Boulder && it.mat.is_building_stone()) {
            if !mats.contains(&it.mat) {
                mats.push(it.mat);
            }
        }
        let mat = *self.rng.pick(&mats);
        let d = Demand { noble, kind, mat, deadline: self.tick + YEAR / 2 };
        let who = self.dwarves.iter().find(|x| x.id == noble).map(|x| (x.name.clone(), x.noble.map_or("noble", |n| n.title())));
        if let Some((name, title)) = who {
            self.msg(&format!("{} the {} demands a {} in their quarters.", name, title.to_lowercase(), d.describe()), (255, 215, 60));
        }
        self.demands.push(d);
    }

    /// Whether a built `kind` of `mat` stands in a bedroom or office the
    /// noble owns, or the enclosed room around their bed.
    pub fn demand_met(&self, m: &Demand) -> bool {
        let mut tiles: HashSet<Pos> = self
            .zones
            .values()
            .filter(|z| z.owner == Some(m.noble) && matches!(z.kind, ZoneKind::Bedroom | ZoneKind::Office))
            .flat_map(|z| z.tiles.iter().copied())
            .collect();
        if let Some(bed) = self.buildings.values().find(|b| b.kind == BuildingKind::Bed && b.owner == Some(m.noble)) {
            if let Some((room, _)) = self.room_at(bed.pos) {
                tiles.extend(room);
            }
        }
        self.buildings.values().any(|b| b.built && b.kind == m.kind && b.mat == m.mat && tiles.contains(&b.pos))
    }

    fn judge_demands(&mut self) {
        let tick = self.tick;
        let demands = std::mem::take(&mut self.demands);
        for m in demands {
            let alive = self.dwarves.iter().any(|d| d.id == m.noble && d.alive);
            if !alive {
                continue;
            }
            if self.demand_met(&m) {
                if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == m.noble) {
                    d.think("is delighted that a demand was satisfied", -30, tick);
                }
                self.msg(&format!("The demand for a {} has been satisfied.", m.describe()), (150, 255, 150));
            } else if tick > m.deadline {
                if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == m.noble) {
                    d.think("is enraged that a demand was ignored", 30, tick);
                }
                self.msg(&format!("The demand for a {} went unmet.", m.describe()), (255, 120, 90));
            } else {
                self.demands.push(m);
            }
        }
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wealth_brings_a_title_and_titled_nobles_make_demands() {
        let mut g = Game::new(81);
        assert_eq!(g.earned_title(), None);
        // Fill the fortress with gold and people.
        let e = g.embark;
        for _ in 0..30 {
            let id = g.spawn_item(ItemKind::Craft, Mat::Gold, e);
            g.items.get_mut(&id).unwrap().quality = 5;
        }
        g.migrants();
        g.migrants();
        g.migrants();
        while g.alive_count() < 20 {
            g.migrants();
        }
        assert!(g.wealth() >= 25_000, "wealth {}", g.wealth());
        assert!(g.earned_title().is_some());
        g.tick = 5 * SEASON + 10;
        g.titles_daily();
        let baron = g.dwarves.iter().find(|d| d.noble.map_or(false, |n| TITLES.iter().any(|t| t.0 == n))).expect("nobody was titled");
        let id = baron.id;
        // Force a demand and satisfy it with a chair in their bedroom zone.
        g.demands.clear();
        g.make_demand(id);
        let m = g.demands[0].clone();
        assert!(!g.demand_met(&m));
        let spot = (2..20).map(|r| e.offset(r, 2, 0)).find(|p| g.can_place(m.kind, *p)).unwrap();
        assert!(g.place_building(m.kind, spot));
        let b = g.world.t(spot).building;
        let bl = g.buildings.get_mut(&b).unwrap();
        bl.built = true;
        bl.mat = m.mat;
        assert!(g.add_zone(spot, spot, ZoneKind::Bedroom) > 0);
        let z = *g.zones.keys().last().unwrap();
        g.zones.get_mut(&z).unwrap().owner = Some(id);
        assert!(g.demand_met(&m));
        // (Past the season's first day, so no new demand comes the same day.)
        g.tick = 5 * SEASON + DAY + 10;
        g.titles_daily();
        assert!(g.demands.iter().all(|d| d.noble != id || d != &m), "a met demand should be cleared");
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(d.thoughts.iter().any(|t| t.text == "is delighted that a demand was satisfied"));
    }

    #[test]
    fn offices_go_to_post_holders_and_help_them_work() {
        let mut g = Game::new(81);
        let e = g.embark;
        let tiles = g.add_zone(e.offset(-3, -3, 0), e.offset(-2, -2, 0), ZoneKind::Office);
        assert!(tiles > 0);
        let office = g.zones.values().find(|z| z.kind == ZoneKind::Office).unwrap().id;
        // The expedition leader may take it; clear titles to test the posts.
        for d in g.dwarves.iter_mut() {
            d.noble = None;
        }
        g.assign_offices();
        assert_eq!(g.zones[&office].owner, None, "nobody needs an office yet");
        let id = g.dwarves[3].id;
        g.dwarves[3].personality.diligence = 20;
        g.dwarves[3].personality.anxiety = 0;
        g.appoint(Position::Bookkeeper, Some(id));
        assert_eq!(g.post_fit(Position::Bookkeeper), Some(Fit::Fair));
        assert_eq!(g.owner_candidates()[0], id, "post holders come first in the owner list");
        g.assign_offices();
        assert_eq!(g.zones[&office].owner, Some(id));
        assert_eq!(g.post_fit(Position::Bookkeeper), Some(Fit::Good), "an office makes the bookkeeper better");
        let (_, notes) = g.post_suitability(Position::Bookkeeper, &g.dwarves[3]);
        assert!(notes.iter().any(|n| n == "has an office"));
    }

    #[test]
    fn candidates_are_ranked_by_suitability() {
        let mut g = Game::new(7);
        let ids: Vec<u32> = g.dwarves.iter().filter(|d| !d.is_child()).map(|d| d.id).collect();
        let (a, b) = (ids[0], ids[1]);
        for d in g.dwarves.iter_mut() {
            d.fight_xp = 0;
            d.personality.temper = 0;
            d.personality.diligence = 0;
        }
        g.dwarves.iter_mut().find(|d| d.id == b).unwrap().fight_xp = 2000;
        let c = g.candidates(Position::Sheriff);
        assert_eq!(c[0], b, "the best fighter should top the sheriff list");
        assert_eq!(c.len(), ids.len());
        let (s, notes) = Position::Sheriff.suitability(g.dwarves.iter().find(|d| d.id == b).unwrap());
        assert_eq!(Fit::of(s), Fit::Good);
        assert!(notes.iter().any(|n| n.contains("fighter")), "{:?}", notes);
        // Taking a second post moves a dwarf off the first, and says so.
        g.appoint(Position::Broker, Some(a));
        let m = g.appoint_msg(Position::Manager, Some(a));
        assert!(m.contains("no longer broker"), "{}", m);
        assert!(g.holder(Position::Broker).is_none());
        assert_eq!(g.appoint_msg(Position::Manager, None), "The post of manager is vacant.");
    }

    #[test]
    fn positions_go_to_one_dwarf_each_and_cycle_to_vacant() {
        let mut g = Game::new(81);
        let a = g.dwarves[0].id;
        g.appoint(Position::Manager, Some(a));
        assert_eq!(g.holder(Position::Manager).map(|d| d.id), Some(a));
        g.appoint(Position::Broker, Some(a));
        assert!(g.holder(Position::Manager).is_none(), "one position per dwarf");
        assert_eq!(g.position_of(a), Some(Position::Broker));
        let n = g.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        let mut vacant = false;
        for _ in 0..=n {
            g.cycle_position(Position::ChiefMedic);
            vacant |= g.holder(Position::ChiefMedic).is_none();
        }
        assert!(vacant, "cycling should pass through vacant");
    }

    #[test]
    fn a_chief_medical_dwarf_and_a_commander_speed_things_up_by_how_well_they_fit() {
        let mut g = Game::new(81);
        let d = g.dwarves[1].clone();
        let eight = |g: &mut Game| {
            (0..8u64).map(|t| {
                g.tick = 1000 + t;
                g.drill_xp(&d)
            }).sum::<u32>()
        };
        let plain = eight(&mut g);
        let cmd = g.dwarves[0].id;
        g.appoint(Position::Commander, Some(cmd));
        // Fighting skill makes the commander: none, some, a veteran.
        for (xp, social, extra, fit) in [(0, -50, 1, Fit::Poor), (400, 0, 2, Fit::Fair), (2000, 0, 4, Fit::Good)] {
            g.dwarves[0].fight_xp = xp;
            g.dwarves[0].personality.sociability = social;
            assert_eq!(g.post_fit(Position::Commander), Some(fit));
            assert_eq!(eight(&mut g), plain + extra, "{:?} commander", fit);
        }
        assert_eq!(g.treat_time(100), 100);
        g.appoint(Position::ChiefMedic, Some(g.dwarves[2].id));
        g.dwarves[2].personality.diligence = 0;
        for (xp, t) in [(0, 90), (800, 75), (2000, 60)] {
            g.dwarves[2].skills[Labor::Medicine.idx()] = xp;
            assert_eq!(g.treat_time(100), t);
        }
        // A holder taking their own turn is judged from the last count.
        g.count_fits();
        let medic = std::mem::take(&mut g.dwarves[2]);
        assert_eq!(g.treat_time(100), 60, "judged from the last count");
        g.dwarves[2] = medic;
    }

    #[test]
    fn a_good_manager_and_bookkeeper_see_to_orders_more_often() {
        let mut g = Game::new(81);
        let (m, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.appoint(Position::Manager, Some(m));
        let passes = |g: &mut Game| (0..DAY).filter(|&t| g.orders_due(t)).count();
        g.dwarves[0].personality.diligence = 0;
        g.dwarves[0].personality.sociability = 0;
        assert_eq!(passes(&mut g), 1);
        g.dwarves[0].personality.diligence = 40;
        assert_eq!(passes(&mut g), 2);
        g.appoint(Position::Bookkeeper, Some(b));
        g.dwarves[1].personality.anxiety = 0;
        for (dil, n) in [(-30, 2), (10, 4), (40, 8)] {
            g.dwarves[1].personality.diligence = dil;
            assert!(passes(&mut g) >= n, "a bookkeeper with diligence {} should count {} times", dil, n);
        }
    }
}
