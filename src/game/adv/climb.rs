//! Going up and down, and digging (G03, the climb package): walking onto a
//! ramp takes the hero up it as in DF, `<` and `>` work on ramps as well as
//! stairs, the sidebar says what the hero's tile allows ("Here:"), the map
//! shows which slopes can really be used, and a hero with a pick in hand
//! can dig through rock (`D`), cut a stair down or up, or steps out of a pit.
//!
//! All of it reads the fort's own rules: `World::neighbors` decides where a
//! ramp or stair leads, and a dig goes through `complete_dig` with the
//! fort's hardness and edge (P10), so a ramp means the same in both modes.

use super::super::*;
use super::turn::Outcome;
use super::Visit;

/// The eight directions' names, for "a ramp up to the north-east".
pub fn dir_word(dx: i32, dy: i32) -> &'static str {
    match (dx.signum(), dy.signum()) {
        (0, -1) => "north",
        (1, -1) => "north-east",
        (1, 0) => "east",
        (1, 1) => "south-east",
        (0, 1) => "south",
        (-1, 1) => "south-west",
        (-1, 0) => "west",
        (-1, -1) => "north-west",
        _ => "here",
    }
}

/// Where the hero can go from `p` by a change of level: straight up or
/// down a stair, or up or down a ramp beside (each a list of the places).
#[derive(Default, Debug)]
pub struct Ways {
    pub stair_up: bool,
    pub stair_down: bool,
    pub ramp_up: Vec<Pos>,
    pub ramp_down: Vec<Pos>,
}

impl Ways {
    pub fn up(&self) -> bool {
        self.stair_up || !self.ramp_up.is_empty()
    }
    pub fn down(&self) -> bool {
        self.stair_down || !self.ramp_down.is_empty()
    }
}

/// The level changes the fort's own walking rules (`World::neighbors`)
/// allow from `p`.
pub fn ways(w: &World, p: Pos) -> Ways {
    let mut nb = Vec::new();
    w.neighbors(p, &mut nb, Mover::Dwarf);
    let mut ways = Ways::default();
    for &(q, _) in &nb {
        let same = q.x == p.x && q.y == p.y;
        match (q.z - p.z, same) {
            (1, true) => ways.stair_up = true,
            (-1, true) => ways.stair_down = true,
            (1, false) => ways.ramp_up.push(q),
            (-1, false) => ways.ramp_down.push(q),
            _ => {}
        }
    }
    ways
}

/// Whether the tile at `p` looks like a way up or down on the map (a ramp,
/// a stair, open space over a ramp), and if so whether someone on `p`'s
/// level could use it from a tile beside it: `Some(true)` for a way that
/// works, `Some(false)` for a slope that leads nowhere (or a ramp seen far
/// below through open air), `None` for anything else.
pub fn usable(w: &World, p: Pos) -> Option<bool> {
    let t = w.get(p)?;
    match t.kind {
        TileKind::Ramp => Some(!ways(w, p).ramp_up.is_empty()),
        TileKind::StairUp | TileKind::StairDown | TileKind::StairUD => {
            let ws = ways(w, p);
            Some(ws.stair_up || ws.stair_down)
        }
        TileKind::Open => {
            // Look down through the open air to what is under it.
            let below = w.kind(p.down())?;
            if below == TileKind::Ramp {
                // Down onto the ramp from a tile beside the open space.
                let r = p.down();
                let ok = DIRS8.iter().any(|&(dx, dy)| {
                    let q = p.offset(dx, dy, 0);
                    w.passable(q, Mover::Dwarf) && w.solid(q.down()) && w.passable(r, Mover::Dwarf)
                });
                return Some(ok);
            }
            // Further down (or a stair's top seen from above): seen, but
            // out of reach from here.
            let mut k = 1;
            while w.kind(p.offset(0, 0, -k)) == Some(TileKind::Open) && k < 15 {
                k += 1;
            }
            match w.kind(p.offset(0, 0, -k)) {
                Some(TileKind::Ramp | TileKind::StairUp | TileKind::StairUD) => Some(false),
                _ => None,
            }
        }
        _ => None,
    }
}

/// What the hero's tile allows, for the sidebar's "Here:" line, and the
/// ways up or down a step away ("A ramp up to the north-east").
pub fn here_lines(w: &World, p: Pos) -> (String, Vec<String>) {
    let ws = ways(w, p);
    let here = if ws.stair_up && ws.stair_down {
        "Stairs: < up, > down"
    } else if ws.stair_up {
        "Stairs: < up"
    } else if ws.stair_down {
        "Stairs: > down"
    } else if !ws.ramp_up.is_empty() {
        "Ramp up: walk uphill, or <"
    } else if !ws.ramp_down.is_empty() {
        "Open space below: > to climb down"
    } else {
        "No way up or down here."
    };
    // A step away: what the tiles the hero can walk to offer that this one
    // doesn't.
    let mut near = Vec::new();
    let mut nb = Vec::new();
    w.neighbors(p, &mut nb, Mover::Dwarf);
    for &(dx, dy) in DIRS8.iter() {
        let q = p.offset(dx, dy, 0);
        if !nb.iter().any(|(n, _)| *n == q) {
            continue;
        }
        let qs = ways(w, q);
        let dir = dir_word(dx, dy);
        let line = match w.kind(q) {
            Some(TileKind::StairUp | TileKind::StairDown | TileKind::StairUD) if (qs.stair_up && !ws.up()) || (qs.stair_down && !ws.down()) => {
                let what = match (qs.stair_up, qs.stair_down) {
                    (true, true) => "up and down",
                    (true, false) => "up",
                    _ => "down",
                };
                format!("Stairs {} to the {}", what, dir)
            }
            Some(TileKind::Ramp) if !qs.ramp_up.is_empty() && !ws.up() => format!("A ramp up to the {}", dir),
            _ => continue,
        };
        if !near.contains(&line) {
            near.push(line);
        }
        if near.len() == 2 {
            break;
        }
    }
    (here.to_string(), near)
}

impl Game {
    /// A step onto a ramp that leads on up in the same direction carries
    /// the hero up it, as in DF: the top it leads to, if any is free.
    pub(crate) fn ramp_top(&self, on: Pos, dx: i32, dy: i32) -> Option<Pos> {
        if self.world.kind(on) != Some(TileKind::Ramp) {
            return None;
        }
        let top = on.offset(dx, dy, 1);
        let ws = ways(&self.world, on);
        ws.ramp_up.contains(&top).then_some(top).filter(|&t| !self.creatures.values().any(|c| c.alive && c.pos == t))
    }

    /// Where `<` (dz 1) or `>` (dz -1) takes the hero from `p`: a stair
    /// first, else a ramp's top, or down onto a ramp beside (the one
    /// nearest the way they were going, `prefer`).
    pub(crate) fn climb_to(&self, p: Pos, dz: i32, prefer: Option<(i32, i32)>) -> Option<Pos> {
        let ws = ways(&self.world, p);
        if dz > 0 && ws.stair_up {
            return Some(p.up());
        }
        if dz < 0 && ws.stair_down {
            return Some(p.down());
        }
        let list = if dz > 0 { ws.ramp_up } else { ws.ramp_down };
        let (px, py) = prefer.unwrap_or((0, 0));
        list.into_iter().min_by_key(|q| {
            let (dx, dy) = (q.x - p.x, q.y - p.y);
            // Straight on first, then orthogonal, then the rest.
            (-(dx * px + dy * py), (dx != 0 && dy != 0) as i32)
        })
    }

    /// The pick in the hero's hand, if any (a mattock is a pick here).
    pub(crate) fn pick_in_hand(&self, hero: usize) -> Option<&Item> {
        self.dwarves[hero].held.and_then(|id| self.items.get(&id)).filter(|it| it.kind == ItemKind::Pick)
    }

    /// The hero digs beside them (`dz` 0), a stair down (-1), or up (1): a
    /// long act, the time the rock's hardness against the pick's edge and
    /// the hero's mining; a foe in sight or a blow stops it with the rock
    /// still whole. Done, the rock is gone the fort's way (`complete_dig`):
    /// stone or ore to carry, perhaps a gem.
    pub(crate) fn hero_dig(&mut self, hero: usize, dx: i32, dy: i32, dz: i32) -> Outcome {
        let Some(edge) = self.pick_in_hand(hero).map(|it| it.mat.edge()) else {
            return Outcome::Refused("You need a pick to dig.".into());
        };
        let plan = match self.dig_plan(hero, dx, dy, dz) {
            Ok(p) => p,
            Err(why) => return Outcome::Refused(why),
        };
        if let Some(why) = self.alarm(hero, self.tick) {
            return Outcome::Refused(format!("You can't dig now: {}", why));
        }
        let level = self.dwarves[hero].skill(Labor::Mining);
        let ticks = match plan.rock {
            Some(m) => Self::work_time(m.dig_ticks_with(edge), level),
            // Cutting steps into a pit's floor: soft work.
            None => Self::work_time(Mat::Loam.dig_ticks_with(edge), level),
        };
        match self.adv_run(hero, ticks, true) {
            Outcome::Done { ticks } => {
                self.dig_done(hero, &plan);
                Outcome::Done { ticks }
            }
            Outcome::Interrupted { ticks, why } => Outcome::Interrupted { ticks, why: format!("You stop digging. {}", why) },
            o => o,
        }
    }

    /// What a dig would change, or why it can't be done.
    fn dig_plan(&self, hero: usize, dx: i32, dy: i32, dz: i32) -> Result<DigPlan, String> {
        let p = self.dwarves[hero].pos;
        let w = &self.world;
        let rock = |q: Pos| -> Result<Mat, String> {
            let Some(t) = w.get(q) else { return Err("There is nothing to dig there.".into()) };
            if t.kind == TileKind::Tree {
                return Err("That is a tree: it wants an axe, not a pick.".into());
            }
            if t.built_from.is_some() {
                return Err("That wall was built, not grown: a pick won't take it apart.".into());
            }
            if !matches!(t.kind, TileKind::Wall | TileKind::Boulder) {
                return Err("There is no rock there to dig.".into());
            }
            if !t.mat.diggable() {
                return Err(format!("The {} is too hard for any pick.", t.mat.name()));
            }
            Ok(t.mat)
        };
        let here = w.t(p).kind;
        let floor_here = matches!(here, TileKind::Floor | TileKind::StairUp | TileKind::StairDown) && w.t(p).building == 0;
        match dz.signum() {
            0 => {
                let q = p.offset(dx.signum(), dy.signum(), 0);
                if q == p {
                    return Err("Dig which way?".into());
                }
                let m = rock(q)?;
                Ok(DigPlan { rock: Some(m), at: q, desig: Desig::Dig, here: None })
            }
            -1 => {
                let q = p.down();
                if w.get(q).map_or(false, |t| !t.kind.solid()) {
                    return Err("There is no rock below to dig.".into());
                }
                let m = rock(q)?;
                if !floor_here {
                    return Err("You can't cut a stair down from where you stand.".into());
                }
                let k = if here == TileKind::StairUp { TileKind::StairUD } else { TileKind::StairDown };
                Ok(DigPlan { rock: Some(m), at: q, desig: Desig::StairUp, here: Some(k) })
            }
            _ => {
                let q = p.up();
                if !floor_here {
                    return Err("You can't cut a stair up from where you stand.".into());
                }
                let k = if here == TileKind::StairDown { TileKind::StairUD } else { TileKind::StairUp };
                match w.kind(q) {
                    // Rock overhead: a stair up through it.
                    Some(TileKind::Wall | TileKind::Boulder) => {
                        let m = rock(q)?;
                        Ok(DigPlan { rock: Some(m), at: q, desig: Desig::StairDown, here: Some(k) })
                    }
                    // Open sky over a pit: steps cut into its side, a ramp
                    // up onto the rim, if there is a rim to climb onto.
                    Some(TileKind::Open) => {
                        // (As `World::neighbors` reads a ramp: rock beside,
                        // and a floor on top of it.)
                        let rim = DIRS8.iter().any(|&(dx, dy)| w.solid(p.offset(dx, dy, 0)) && w.passable(p.offset(dx, dy, 1), Mover::Dwarf));
                        if !rim {
                            return Err("There is nothing to climb onto here.".into());
                        }
                        Ok(DigPlan { rock: None, at: p, desig: Desig::Ramp, here: None })
                    }
                    _ => Err("There is no rock above to dig.".into()),
                }
            }
        }
    }

    /// The dig is done: the fort's own `complete_dig`, the stair's other
    /// half, the miner's practice, and a gem now and then.
    fn dig_done(&mut self, hero: usize, plan: &DigPlan) {
        let p = self.dwarves[hero].pos;
        let first = self.dwarves[hero].name.split(' ').next().unwrap_or("").to_string();
        if plan.desig == Desig::Ramp {
            // (Not `complete_dig`: the hero's own floor becomes the ramp.)
            let t = self.world.t_mut(p);
            t.kind = TileKind::Ramp;
            t.dug = true;
            self.dig_version += 1;
            self.msg(&format!("{} cuts steps into the side of the pit.", first), (220, 200, 150));
        } else {
            let was = *self.world.t(plan.at);
            self.world.t_mut(plan.at).desig = plan.desig;
            self.complete_dig(plan.at);
            if let Some(k) = plan.here {
                let t = self.world.t_mut(p);
                t.kind = k;
                t.dug = true;
            }
            let what = match plan.desig {
                Desig::StairUp => "a stair down",
                Desig::StairDown => "a stair up",
                _ => "through",
            };
            self.msg(&format!("{} digs {} into the {}.", first, what, was.mat.name()), (220, 200, 150));
            // The stone's own gem, now and then (the fort's veins hold them
            // as walls; a hero's luck turns one out of plain rock).
            if let Some(gem) = was.mat.host_gem().filter(|_| was.kind == TileKind::Wall && self.rng.chance(0.08)) {
                self.spawn_item(ItemKind::Boulder, gem, p);
                self.msg(&format!("{} turns up a rough {}!", first, gem.name()), (210, 160, 255));
            }
        }
        self.dwarves[hero].gain_xp(Labor::Mining, 25);
    }
}

/// One dig: the tile cut (`at`, the way the fort designates it), the rock
/// (None for steps cut in a pit) and what the hero's own tile becomes.
struct DigPlan {
    rock: Option<Mat>,
    at: Pos,
    desig: Desig,
    here: Option<TileKind>,
}

/// The tiles a dig from `p` changes, for the site's memory.
pub fn dig_tiles(p: Pos, dx: i32, dy: i32, dz: i32) -> Vec<Pos> {
    match dz.signum() {
        0 => vec![p.offset(dx.signum(), dy.signum(), 0)],
        d => vec![p, p.offset(0, 0, d)],
    }
}

impl Visit {
    /// The hero digs (`Game::hero_dig`), and the site remembers what was
    /// dug: a regenerated site has it laid back over it, a fallen fort's
    /// save is written back with it (A5).
    pub fn dig(&mut self, dx: i32, dy: i32, dz: i32) -> Outcome {
        let p = self.g.dwarves[self.hero].pos;
        let out = self.g.hero_dig(self.hero, dx, dy, dz);
        if matches!(out, Outcome::Done { .. }) {
            for q in dig_tiles(p, dx, dy, dz) {
                let k = self.g.world.t(q).kind;
                self.dug.retain(|(d, _)| *d != q);
                self.dug.push((q, k));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::test_hero;
    use super::super::turn::Action;
    use super::super::{Adventure, SiteKey, HERO_ID};
    use super::*;

    /// An adventure in a wild field, the hero at its middle on a flat
    /// patch of ground: floor, rock under it, sky over it.
    fn field(seed: u64) -> Adventure {
        static WORLD: std::sync::OnceLock<crate::overworld::Overworld> = std::sync::OnceLock::new();
        let ow = WORLD.get_or_init(|| crate::overworld::Overworld::generate(5)).clone();
        let mut a = Adventure::new(ow, seed, test_hero(seed), None);
        let (x, y) = a.at;
        a.enter(SiteKey::Wild(x, y)).unwrap();
        let v = a.visit.as_mut().unwrap();
        let p = v.g.dwarves[v.hero].pos;
        // No wildlife to get in the way.
        v.g.creatures.clear();
        for dy in -4..=4 {
            for dx in -4..=4 {
                set(&mut v.g, p.offset(dx, dy, -1), TileKind::Wall);
                set(&mut v.g, p.offset(dx, dy, 0), TileKind::Floor);
                set(&mut v.g, p.offset(dx, dy, 1), TileKind::Open);
                set(&mut v.g, p.offset(dx, dy, 2), TileKind::Open);
            }
        }
        a
    }

    fn set(g: &mut Game, p: Pos, k: TileKind) {
        let t = g.world.t_mut(p);
        t.kind = k;
        t.mat = Mat::Granite;
        t.water = 0;
        t.building = 0;
        t.built_from = None;
        t.revealed = true;
    }

    /// A step of hillside east of `p`: a ramp beside it, then the rock it
    /// leans on with its top a level up.
    fn hill(g: &mut Game, p: Pos) {
        set(g, p.offset(1, 0, 0), TileKind::Ramp);
        for dy in -4..=4 {
            set(g, p.offset(2, dy, 0), TileKind::Wall);
            set(g, p.offset(2, dy, 1), TileKind::Floor);
            set(g, p.offset(3, dy, 0), TileKind::Wall);
            set(g, p.offset(3, dy, 1), TileKind::Floor);
        }
    }

    fn give_pick(g: &mut Game, h: usize, mat: Mat) {
        let p = g.dwarves[h].pos;
        let id = g.spawn_item(ItemKind::Pick, mat, p);
        g.items.get_mut(&id).unwrap().carried_by = Some(HERO_ID);
        g.dwarves[h].held = Some(id);
    }

    #[test]
    fn walking_onto_a_ramp_climbs_it_and_off_the_top_comes_down() {
        let mut a = field(31);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        let p = g.dwarves[h].pos;
        hill(g, p);
        // East onto the ramp: up it, onto the rock's top beyond.
        assert!(matches!(g.act(h, Action::Move(1, 0)), Outcome::Done { .. }));
        assert_eq!(g.dwarves[h].pos, p.offset(2, 0, 1), "up the ramp in one step");
        // West into the open space over the ramp: down onto it.
        assert!(matches!(g.act(h, Action::Move(-1, 0)), Outcome::Done { .. }));
        assert_eq!(g.dwarves[h].pos, p.offset(1, 0, 0), "down onto the ramp");
        assert!(matches!(g.act(h, Action::Move(-1, 0)), Outcome::Done { .. }));
        assert_eq!(g.dwarves[h].pos, p);
        // Along the slope (not into the rock): onto the ramp, and no further.
        g.dwarves[h].pos = p.offset(1, 1, 0);
        assert!(matches!(g.act(h, Action::Move(0, -1)), Outcome::Done { .. }));
        assert_eq!(g.dwarves[h].pos, p.offset(1, 0, 0));
        // `<` on the ramp, `>` from its top.
        assert!(matches!(g.act(h, Action::Climb(1)), Outcome::Done { .. }));
        assert_eq!(g.dwarves[h].pos.z, p.z + 1);
        assert!(matches!(g.act(h, Action::Climb(-1)), Outcome::Done { .. }));
        assert_eq!(g.dwarves[h].pos, p.offset(1, 0, 0));
        // `<` on flat ground goes nowhere.
        g.dwarves[h].pos = p.offset(-2, 0, 0);
        assert!(matches!(g.act(h, Action::Climb(1)), Outcome::Refused(_)));
    }

    #[test]
    fn the_here_line_says_what_the_tile_allows() {
        let mut a = field(32);
        let v = a.visit.as_mut().unwrap();
        let g = &mut v.g;
        let p = g.dwarves[v.hero].pos;
        let w = |g: &Game, q: Pos| here_lines(&g.world, q);
        assert_eq!(w(g, p), ("No way up or down here.".to_string(), vec![]));
        hill(g, p);
        assert_eq!(w(g, p), ("No way up or down here.".to_string(), vec!["A ramp up to the east".to_string()]));
        assert_eq!(w(g, p.offset(1, 0, 0)).0, "Ramp up: walk uphill, or <");
        assert_eq!(w(g, p.offset(2, 0, 1)).0, "Open space below: > to climb down");
        // Stairs, up and down.
        let s = p.offset(-2, 0, 0);
        set(g, s, TileKind::StairUD);
        set(g, s.up(), TileKind::StairDown);
        set(g, s.down(), TileKind::StairUp);
        assert_eq!(w(g, s).0, "Stairs: < up, > down");
        assert!(w(g, p.offset(-1, 0, 0)).1.contains(&"Stairs up and down to the west".to_string()), "{:?}", w(g, p.offset(-1, 0, 0)));
        // The map: the ramp that leads up is a way; one against nothing isn't.
        assert_eq!(usable(&g.world, p.offset(1, 0, 0)), Some(true));
        assert_eq!(usable(&g.world, p.offset(1, 0, 1)), Some(true), "the open space over it, from the top");
        set(g, p.offset(-1, 2, 0), TileKind::Ramp);
        assert_eq!(usable(&g.world, p.offset(-1, 2, 0)), Some(false), "a slope with no rock to climb onto");
        assert_eq!(usable(&g.world, p), None);
        assert_eq!(usable(&g.world, s), Some(true));
    }

    #[test]
    fn a_hero_with_a_pick_digs_a_tunnel_and_without_one_cannot() {
        let mut a = field(33);
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        let p = v.g.dwarves[h].pos;
        for i in 1..=3 {
            set(&mut v.g, p.offset(i, 0, 0), TileKind::Wall);
        }
        assert_eq!(v.dig(1, 0, 0), Outcome::Refused("You need a pick to dig.".into()));
        give_pick(&mut v.g, h, Mat::Iron);
        let t0 = v.g.tick;
        for i in 1..=3 {
            let out = v.dig(1, 0, 0);
            assert!(matches!(out, Outcome::Done { .. }), "{:?}", out);
            assert_eq!(v.g.world.t(p.offset(i, 0, 0)).kind, TileKind::Floor);
            assert!(matches!(v.g.act(h, Action::Move(1, 0)), Outcome::Done { .. }));
        }
        assert_eq!(v.g.dwarves[h].pos, p.offset(3, 0, 0), "walked the tunnel");
        // Granite with an iron pick: slow work.
        let took = v.g.tick - t0;
        assert!(took >= 3 * 50, "{} ticks", took);
        assert_eq!(v.dug.len(), 3, "the site remembers the tunnel");
        // Nothing to dig in the open, nor the bedrock.
        assert!(matches!(v.dig(0, 1, 0), Outcome::Refused(_)));
        let q = v.g.dwarves[h].pos.offset(1, 0, 0);
        set(&mut v.g, q, TileKind::Wall);
        v.g.world.t_mut(q).mat = Mat::Bedrock;
        assert!(matches!(v.dig(1, 0, 0), Outcome::Refused(_)));
        assert!(v.g.dwarves[h].skills[Labor::Mining.idx()] > 0, "the hero learns to mine");
    }

    #[test]
    fn digging_down_makes_a_way_down_and_up_gets_out_of_a_pit() {
        let mut a = field(34);
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        let p = v.g.dwarves[h].pos;
        give_pick(&mut v.g, h, Mat::Steel);
        // Down: a stair here, and its foot below.
        assert!(matches!(v.dig(0, 0, -1), Outcome::Done { .. }));
        assert_eq!(v.g.world.t(p).kind, TileKind::StairDown);
        assert_eq!(v.g.world.t(p.down()).kind, TileKind::StairUp);
        assert_eq!(here_lines(&v.g.world, p).0, "Stairs: > down");
        assert!(matches!(v.g.act(h, Action::Climb(-1)), Outcome::Done { .. }));
        assert_eq!(v.g.dwarves[h].pos, p.down());
        assert!(matches!(v.g.act(h, Action::Climb(1)), Outcome::Done { .. }));
        // A pit: rock all round at the hero's level, the sky over it.
        let pit = p.offset(-3, 0, 0);
        for &(dx, dy) in DIRS8.iter() {
            set(&mut v.g, pit.offset(dx, dy, 0), TileKind::Wall);
            set(&mut v.g, pit.offset(dx, dy, 1), TileKind::Floor);
        }
        v.g.dwarves[h].pos = pit;
        assert_eq!(here_lines(&v.g.world, pit).0, "No way up or down here.");
        assert!(matches!(v.dig(0, 0, 1), Outcome::Done { .. }), "steps cut in the side of the pit");
        assert_eq!(here_lines(&v.g.world, pit).0, "Ramp up: walk uphill, or <");
        assert!(matches!(v.g.act(h, Action::Climb(1)), Outcome::Done { .. }));
        assert_eq!(v.g.dwarves[h].pos.z, pit.z + 1, "out of the pit");
    }

    #[test]
    fn a_foe_stops_the_digging() {
        let mut a = field(35);
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        let p = v.g.dwarves[h].pos;
        set(&mut v.g, p.offset(1, 0, 0), TileKind::Wall);
        v.g.world.t_mut(p.offset(1, 0, 0)).mat = Mat::Obsidian;
        give_pick(&mut v.g, h, Mat::Copper);
        let gob = v.g.spawn_creature(crate::creature::Species::Goblin, p.offset(-3, 0, 0), DAY * 10);
        if let Some(c) = v.g.creatures.get_mut(&gob) {
            c.siege = true;
            c.target = Some(HERO_ID);
        }
        assert!(matches!(v.dig(1, 0, 0), Outcome::Refused(_)), "not with a goblin in sight");
        v.g.creatures.get_mut(&gob).unwrap().pos = p.offset(-30, 0, 0);
        let out = v.dig(1, 0, 0);
        assert!(matches!(out, Outcome::Interrupted { .. } | Outcome::Died), "{:?}", out);
        assert_eq!(v.g.world.t(p.offset(1, 0, 0)).kind, TileKind::Wall, "the rock still whole");
        assert!(v.dug.is_empty());
    }

    #[test]
    fn a_site_remembers_what_was_dug() {
        let mut a = field(36);
        let key = a.visit.as_ref().unwrap().key;
        let (q, below) = {
            let v = a.visit.as_mut().unwrap();
            let h = v.hero;
            let p = v.g.dwarves[h].pos;
            set(&mut v.g, p.offset(0, 1, 0), TileKind::Wall);
            give_pick(&mut v.g, h, Mat::Steel);
            assert!(matches!(v.dig(0, 1, 0), Outcome::Done { .. }));
            assert!(matches!(v.dig(0, 0, -1), Outcome::Done { .. }));
            (p.offset(0, 1, 0), p)
        };
        a.leave();
        assert_eq!(a.memos.get(&key).map(|m| m.dug.len()), Some(3));
        a.enter(key).unwrap();
        let g = &a.visit.as_ref().unwrap().g;
        assert_eq!(g.world.t(q).kind, TileKind::Floor, "the tunnel is still there");
        assert_eq!(g.world.t(below).kind, TileKind::StairDown);
        assert_eq!(g.world.t(below.down()).kind, TileKind::StairUp);
    }
}
