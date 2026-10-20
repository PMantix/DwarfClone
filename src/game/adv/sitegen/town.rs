//! A living town, hamlet, fortress or retreat: houses about a hall whose
//! walls are carved with the place's history, named folk who remember it,
//! and guards. How the folk take the hero is the race's stance
//! (`peoples::stance`): at home, among friends, watched, or hunted by the
//! guards. Strike one of the folk and the guards turn on you.
//!
//! Folk are `Visitor`s with the `FOLK` role and guards `Visitor`s with the
//! `GUARD` role, run by `folk_tick` (one line in `creature_tick`): they
//! wander near home, stay in at night, flee danger, and the guards fight
//! it. A guard who turns on the hero becomes a soldier of their people,
//! hunting the hero (`siege`, `target`) by the fort's own code.

use super::super::peoples::Stance;
use super::super::HERO_ID;
use super::{approach, arm, civ_race, hashed_name, lore, occupant, soldier, Ctx, Made};
use crate::creature::{Creature, Species};
use crate::game::combat::{adjacent, dist2, near};
use crate::game::{clock, detail, Game, DAY};
use crate::geom::Pos;
use crate::items::ItemKind;
use crate::material::Mat;
use crate::overworld::{Race, Site, SiteKind};
use crate::rng::Rng;
use crate::world::TileKind;

/// The `Creature.visitor` role of a townsperson (the fort's visitor roles
/// are below 10, so no fort creature ever has these).
pub const FOLK: u8 = 20;
/// A town guard's role.
pub const GUARD: u8 = 21;

/// What the folk do for a living.
const ROLES: [&str; 10] = ["smith", "brewer", "hunter", "bard", "farmer", "weaver", "cook", "mason", "herbalist", "carpenter"];

/// A building's outline: walls on its border, floor within.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub z: i32,
}

impl Rect {
    pub fn inside(&self) -> Vec<Pos> {
        let mut v = Vec::new();
        for y in self.y0 + 1..self.y1 {
            for x in self.x0 + 1..self.x1 {
                v.push(Pos::new(x, y, self.z));
            }
        }
        v
    }

    pub fn border(&self) -> Vec<Pos> {
        let mut v = Vec::new();
        for y in self.y0..=self.y1 {
            for x in self.x0..=self.x1 {
                if x == self.x0 || x == self.x1 || y == self.y0 || y == self.y1 {
                    v.push(Pos::new(x, y, self.z));
                }
            }
        }
        v
    }

    pub fn mid(&self) -> Pos {
        Pos::new((self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2, self.z)
    }

    fn overlaps(&self, o: &Rect, gap: i32) -> bool {
        self.x0 - gap <= o.x1 && o.x0 - gap <= self.x1 && self.y0 - gap <= o.y1 && o.y0 - gap <= self.y1
    }
}

/// Where a site's buildings stand.
#[derive(Default)]
pub struct Plan {
    pub hall: Option<Rect>,
    pub houses: Vec<Rect>,
    /// The doorways (kept clear of rubble and folk).
    pub doors: Vec<Pos>,
}

/// What a people builds with.
pub fn build_mat(race: Option<Race>, kind: SiteKind) -> Mat {
    match (race, kind) {
        (Some(Race::Elf), _) | (_, SiteKind::Retreat) => Mat::Wood,
        (Some(Race::Goblin), _) | (_, SiteKind::DarkFortress) => Mat::Basalt,
        (Some(Race::Dwarf), _) | (_, SiteKind::Fortress) => Mat::Granite,
        (_, SiteKind::Hamlet) => Mat::Wood,
        _ => Mat::Limestone,
    }
}

/// How many houses and how big, and whether a hall, by the site's kind.
fn shape(kind: SiteKind, rng: &mut Rng) -> (Option<(i32, i32)>, usize, (i32, i32)) {
    match kind {
        SiteKind::Hamlet => (Some((6, 5)), rng.range(5, 9) as usize, (6, 5)),
        SiteKind::Town => (Some((10, 7)), rng.range(8, 13) as usize, (7, 6)),
        SiteKind::Fortress => (Some((12, 8)), rng.range(4, 7) as usize, (6, 5)),
        SiteKind::Retreat => (Some((7, 7)), rng.range(4, 7) as usize, (5, 5)),
        SiteKind::DarkFortress => (Some((9, 9)), 4, (5, 5)),
        SiteKind::Lair => (Some((7, 7)), 0, (5, 5)),
    }
}

/// Raise a site's buildings about `center` on level ground: a hall in the
/// middle and houses round it, of `mat`, each with a doorway.
pub fn lay_out(g: &mut Game, center: Pos, kind: SiteKind, mat: Mat, rng: &mut Rng) -> Plan {
    let (hall, n, (hw, hh)) = shape(kind, rng);
    let mut plan = Plan::default();
    let mut taken: Vec<Rect> = Vec::new();
    if let Some((w, h)) = hall {
        plan.hall = place(g, center, 5, w, h, &taken, rng);
        taken.extend(plan.hall);
    }
    for _ in 0..n {
        let (w, h) = (rng.range(hw - 2, hw + 1).max(4), rng.range(hh - 1, hh + 1).max(4));
        if let Some(r) = place(g, center, 18, w, h, &taken, rng) {
            taken.push(r);
            plan.houses.push(r);
        }
    }
    for r in taken.iter() {
        let border = r.border();
        // The door: the middle of the side facing the centre.
        let door = if (center.y - r.mid().y).abs() >= (center.x - r.mid().x).abs() {
            Pos::new(r.mid().x, if center.y > r.mid().y { r.y1 } else { r.y0 }, r.z)
        } else {
            Pos::new(if center.x > r.mid().x { r.x1 } else { r.x0 }, r.mid().y, r.z)
        };
        let ground = g.world.get(r.mid().down()).map_or(Mat::Loam, |t| t.mat);
        for p in r.inside() {
            let t = g.world.t_mut(p);
            t.kind = TileKind::Floor;
            if t.mat == Mat::Wood || t.mat == Mat::Air {
                t.mat = ground;
            }
        }
        for p in border {
            if p == door {
                let t = g.world.t_mut(p);
                t.kind = TileKind::Floor;
                continue;
            }
            let t = g.world.t_mut(p);
            t.kind = TileKind::Wall;
            t.mat = mat;
            t.built_from = Some(ItemKind::Block);
            t.revealed = true;
        }
        plan.doors.push(door);
    }
    g.dig_version += 1;
    plan
}

/// A spot for a `w`×`h` building within `r` of `center`: level, dry, open
/// ground, clear of the map's edge and of the others.
fn place(g: &Game, center: Pos, r: i32, w: i32, h: i32, taken: &[Rect], rng: &mut Rng) -> Option<Rect> {
    let world = &g.world;
    for _ in 0..200 {
        let x0 = center.x + rng.range(-r, r + 1) - w / 2;
        let y0 = center.y + rng.range(-r, r + 1) - h / 2;
        let (x1, y1) = (x0 + w - 1, y0 + h - 1);
        if x0 < 3 || y0 < 3 || x1 >= world.w - 3 || y1 >= world.h - 3 {
            continue;
        }
        let z = world.surface_z(x0, y0);
        let rect = Rect { x0, y0, x1, y1, z };
        if taken.iter().any(|o| rect.overlaps(o, 2)) {
            continue;
        }
        // The one arriving at the centre isn't walled in.
        if center.x >= x0 - 1 && center.x <= x1 + 1 && center.y >= y0 - 1 && center.y <= y1 + 1 && taken.is_empty() && r > 5 {
            continue;
        }
        let level = (y0..=y1).all(|y| {
            (x0..=x1).all(|x| {
                let p = Pos::new(x, y, z);
                world.surface_z(x, y) == z
                    && world.get(p).map_or(false, |t| matches!(t.kind, TileKind::Floor | TileKind::Tree | TileKind::Shrub | TileKind::Boulder) && t.water == 0 && t.magma == 0 && !t.frozen)
                    && world.solid(p.down())
            })
        });
        if level {
            return Some(rect);
        }
    }
    None
}

/// Carve the place's history into its hall's walls (the look panel reads
/// an engraving's legend).
pub fn engrave(g: &mut Game, r: &Rect, events: &[usize]) {
    if events.is_empty() {
        return;
    }
    for (i, p) in r.border().into_iter().enumerate() {
        if g.world.get(p).map_or(false, |t| t.kind == TileKind::Wall) {
            let e = events[i % events.len()];
            let t = g.world.t_mut(p);
            t.smooth = detail::ENGRAVED + 1;
            t.engraving = e as u32 + 1;
        }
    }
}

/// A living site: its buildings, its folk and its guards.
pub fn make(g: &mut Game, ctx: &Ctx, site: &Site, stance: Stance) -> Made {
    let mut made = Made::default();
    let (title, story) = lore::town_story(&g.overworld, site, stance);
    made.title = title;
    made.story = story;
    made.hostile = stance == Stance::Hostile;
    let mut rng = Rng::new(ctx.seed ^ 0x7055);
    let race = civ_race(&g.overworld, site.civ).unwrap_or(Race::Human);
    let center = g.embark;
    let plan = lay_out(g, center, site.kind, build_mat(Some(race), site.kind), &mut rng);
    let events: Vec<(usize, i32, String)> = lore::about(&g.overworld, &site.name).into_iter().map(|(i, y, t)| (i, y, t.to_string())).collect();
    if let Some(h) = plan.hall {
        engrave(g, &h, &events.iter().map(|e| e.0).collect::<Vec<_>>());
        // Ale in the hall.
        for _ in 0..rng.range(1, 4) {
            let p = *rng.pick(&h.inside());
            super::loot(g, &mut made, ItemKind::Drink, Mat::Wood, p, 0, None);
        }
    }
    made.arrive = Some(approach(&g.world, center, 9, 15, &mut rng));
    // The folk, at home.
    let homes: Vec<Pos> = plan.houses.iter().chain(plan.hall.iter()).flat_map(|r| r.inside()).collect();
    let n = (site.pop / 15).clamp(6, 14) as usize;
    for i in 0..n {
        let at = if homes.is_empty() { g.spot_near(center, 8) } else { *rng.pick(&homes) };
        let name = hashed_name(ctx.seed, i as u64, race);
        let role = if i == 0 { "elder" } else { ROLES[(crate::game::names::hash(&[ctx.seed, i as u64, 7]) % ROLES.len() as u64) as usize] };
        let id = occupant(g, &mut made, Species::Visitor, at, Some(format!("{}, {}", name, role)), false);
        // Some remember the place's past.
        let memory = (!events.is_empty() && i % 2 == 0).then(|| &events[(crate::game::names::hash(&[ctx.seed, i as u64, 11]) % events.len() as u64) as usize]);
        if let Some(c) = g.creatures.get_mut(&id) {
            c.visitor = FOLK;
            c.desc = Some(match memory {
                Some((_, y, t)) => format!("The {} of {}. Remembers {}: {}", role, site.name, lore::in_year(*y), t),
                None => format!("The {} of {}.", role, site.name),
            });
            if stance == Stance::Hostile {
                c.wary_until = u64::MAX;
            }
        }
    }
    // The guards: at the hall's door, or come for the hero.
    let guards = rng.range(2, 4) + (site.pop > 120) as i32;
    let post = plan.doors.first().copied().unwrap_or(center);
    for i in 0..guards {
        let at = g.spot_near(post, 2);
        let name = hashed_name(ctx.seed, 100 + i as u64, race);
        let hostile = stance == Stance::Hostile;
        let sp = if hostile { soldier(race) } else { Species::Visitor };
        let id = occupant(g, &mut made, sp, at, Some(format!("{}, guard of {}", name, site.name)), hostile);
        if let Some(c) = g.creatures.get_mut(&id) {
            c.visitor = GUARD;
            c.desc = Some(format!("Keeps the peace of {}.", site.name));
        }
        let weapon = match race {
            Race::Dwarf => ItemKind::Axe,
            Race::Human => ItemKind::Sword,
            _ => ItemKind::Spear,
        };
        let mat = if race == Race::Elf { Mat::Wood } else { Mat::Iron };
        arm(g, id, weapon, mat);
        arm(g, id, ItemKind::Helm, if race == Race::Elf { Mat::Wood } else { Mat::Copper });
    }
    made
}

impl Game {
    /// A townsperson's or a guard's tick (from `creature_tick`): `Some`
    /// when it is done, `None` to let the usual code run (a guard who has
    /// turned on the hero hunts them as any soldier would).
    pub(crate) fn folk_tick(&mut self, c: &mut Creature) -> Option<bool> {
        if c.siege {
            return None;
        }
        c.attack_cd = c.attack_cd.saturating_sub(1);
        c.move_cd = c.move_cd.saturating_sub(1);
        // Struck by the hero (`dwarf_attack` makes the striker its target).
        if c.target == Some(HERO_ID) {
            c.target = None;
            self.raise_the_guard(c);
            if c.siege {
                return None;
            }
        }
        let info = c.species.info();
        let me = c.pos;
        let foe = self
            .creatures
            .values()
            .filter(|o| o.alive && o.hostile() && !o.tame && near(o.pos, me, 8) && !(o.visitor >= FOLK && !o.siege))
            .min_by_key(|o| dist2(o.pos, me))
            .map(|o| (o.id, o.pos));
        if c.visitor == GUARD {
            if let Some((oid, op)) = foe {
                c.foe = Some(oid);
                if adjacent(op, me) {
                    if c.attack_cd == 0 {
                        c.attack_cd = info.attack_delay;
                        self.creature_maul(c, oid);
                    }
                } else {
                    self.pursue_foe(c);
                }
                return Some(true);
            }
            c.foe = None;
        } else {
            // Folk run from danger (and from a hero who has shed blood here).
            let hero = (c.wary_until > self.tick).then(|| self.dwarves.iter().find(|d| d.alive && d.id == HERO_ID && near(d.pos, me, 6)).map(|d| d.pos)).flatten();
            if let Some(t) = foe.map(|f| f.1).or(hero) {
                let mut nb = Vec::new();
                self.world.neighbors(me, &mut nb, info.mover);
                if let Some(&(best, _)) = nb.iter().max_by_key(|(q, _)| dist2(*q, t)) {
                    if dist2(best, t) > dist2(me, t) {
                        c.path = vec![best];
                    }
                }
                self.creature_step(c, info.move_delay);
                return Some(true);
            }
        }
        // About their day; indoors at night.
        if c.path.is_empty() && !clock::is_night(self.tick) && self.rng.chance(0.01) {
            self.creature_wander(c, 6);
        }
        self.creature_step(c, info.move_delay + 2);
        Some(true)
    }

    /// The hero struck one of the folk: the guards turn soldier and come
    /// for them, and the folk keep away.
    fn raise_the_guard(&mut self, c: &mut Creature) {
        let site = self.overworld.site_at(self.site.0, self.site.1).cloned();
        let race = site.as_ref().and_then(|s| civ_race(&self.overworld, s.civ)).unwrap_or(Race::Human);
        let sp = soldier(race);
        let until = self.tick + DAY * 3;
        let turn = |o: &mut Creature| {
            if o.visitor == GUARD && o.alive && !o.siege {
                o.species = sp;
                o.siege = true;
                o.target = Some(HERO_ID);
                o.path.clear();
            } else if o.visitor == FOLK {
                o.wary_until = o.wary_until.max(until);
            }
        };
        turn(c);
        for o in self.creatures.values_mut() {
            turn(o);
        }
        let name = site.map_or_else(|| self.fortress_name.clone(), |s| s.name);
        self.msg(&format!("Blood is shed in {}! Its guards turn on you.", name), (255, 80, 70));
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::super::tests::test_hero;
    use super::super::super::turn::Action;
    use super::super::super::{Adventure, SiteKey, HERO_ID};
    use super::*;
    use crate::overworld::Overworld;

    pub(crate) fn world() -> Overworld {
        static WORLD: std::sync::OnceLock<Overworld> = std::sync::OnceLock::new();
        WORLD.get_or_init(|| Overworld::generate(5)).clone()
    }

    /// An adventure by a hero of `race` (and civ), entered into `site`.
    pub(crate) fn visit(site: u32, race: Race, civ: Option<u32>, seed: u64) -> Adventure {
        let ow = world();
        let mut h = test_hero(seed);
        h.race = race;
        h.civ = civ;
        let mut a = Adventure::new(ow, seed, h, None);
        a.enter(SiteKey::Site(site)).unwrap();
        a
    }

    fn living(ow: &Overworld, race: Race) -> Option<Site> {
        ow.sites.iter().find(|s| !s.ruined && s.kind != SiteKind::Lair && s.civ.and_then(|c| ow.civs.get(c as usize)).map_or(false, |c| c.race == race)).cloned()
    }

    #[test]
    fn townsfolk_are_peaceful_and_guards_fight() {
        let ow = world();
        let s = living(&ow, Race::Dwarf).or_else(|| living(&ow, Race::Human)).expect("a living site");
        let mut a = visit(s.id, ow.civs[s.civ.unwrap() as usize].race, s.civ, 3);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        let folk: Vec<u32> = g.creatures.values().filter(|c| c.visitor == FOLK).map(|c| c.id).collect();
        let guards: Vec<u32> = g.creatures.values().filter(|c| c.visitor == GUARD).map(|c| c.id).collect();
        assert!(folk.len() >= 6, "{} folk", folk.len());
        assert!(guards.len() >= 2);
        assert!(g.creatures.values().all(|c| !c.hostile()), "nobody here is hostile to one of their own");
        assert!(g.world.tiles.iter().any(|t| t.built_from.is_some()), "it has buildings");
        // A wolf in the street: the guards go for it, the folk run.
        let gp = g.creatures[&guards[0]].pos;
        let wp = g.spot_near(gp, 3);
        let wolf = g.spawn_creature(Species::Wolf, wp, DAY);
        let mut fought = false;
        for _ in 0..60 {
            g.act(h, Action::Wait(10));
            let w = g.creatures.get(&wolf);
            if w.map_or(true, |w| !w.alive || w.body.hp.iter().sum::<i32>() < crate::body::Body::new(Species::Wolf.info().scale).hp.iter().sum::<i32>()) {
                fought = true;
                break;
            }
            if guards.iter().any(|id| g.creatures.get(id).map_or(false, |c| c.foe == Some(wolf))) && w.map_or(false, |w| w.foe.is_some()) {
                fought = true;
                break;
            }
        }
        assert!(fought, "the guards fought the wolf");
        // The hero strikes one of the folk: the guards turn on them.
        let f = folk[0];
        let fp = g.creatures[&f].pos;
        let spot = g.spot_near(fp, 1);
        g.dwarves[h].pos = spot;
        g.dwarves[h].pos = if adjacent(spot, fp) && spot != fp { spot } else { fp.offset(1, 0, 0) };
        g.act(h, Action::Attack { target: super::super::super::turn::Target::Creature(f), part: None });
        for _ in 0..5 {
            g.act(h, Action::Wait(10));
        }
        assert!(g.creatures.values().filter(|c| c.visitor == GUARD && c.alive).all(|c| c.siege && c.target == Some(HERO_ID) && c.hostile()), "the guards turned");
        // And leaving, the hero is known for it.
        a.leave();
        assert!(a.fame.infamy > 0, "infamy");
    }

    #[test]
    fn a_goblin_is_not_welcome_in_a_dwarf_hamlet() {
        let ow = world();
        let Some(s) = living(&ow, Race::Dwarf).or_else(|| living(&ow, Race::Human)) else { return };
        let race = ow.civs[s.civ.unwrap() as usize].race;
        // A goblin: the guards come for them at once.
        let a = visit(s.id, Race::Goblin, None, 4);
        let v = a.visit.as_ref().unwrap();
        let guards: Vec<&Creature> = v.g.creatures.values().filter(|c| c.visitor == GUARD).collect();
        assert!(!guards.is_empty());
        assert!(guards.iter().all(|c| c.siege && c.target == Some(HERO_ID) && c.hostile()));
        assert!(v.g.log.iter().any(|e| e.text.contains("enemy")), "{:?}", v.g.log.iter().map(|e| &e.text).collect::<Vec<_>>());
        // One of their own walks in in peace.
        let a = visit(s.id, race, s.civ, 4);
        let v = a.visit.as_ref().unwrap();
        assert!(v.g.creatures.values().all(|c| !c.siege && !c.hostile()));
    }
}
