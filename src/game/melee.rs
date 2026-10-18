//! Close quarters: how creatures close with a foe and stay engaged. One
//! with a foe beside it fights rather than walks; pursuit goes straight for
//! a free tile beside the quarry, so several attackers surround one rather
//! than pile onto a single tile; and beasts, armies, the dead and the
//! monsters of legend fight one another where they're hostile (see
//! `combat::faction`). Breaking off happens only by fleeing or falling.

use super::combat::{adjacent, dist2, faction, hostile_to, near, Faction};
use super::gore::{mix, owner_of_creature};
use super::*;
use crate::creature::{Creature, Species};

/// "a goblin", "an elk", or its own name (for the witnesses).
pub(super) fn a_or_name(c: &Creature) -> String {
    if let Some(l) = &c.label {
        return l.clone();
    }
    let n = c.name();
    let an = n.starts_with(|ch: char| "aeiou".contains(ch));
    format!("{} {}", if an { "an" } else { "a" }, n)
}

impl Game {
    /// A foe beside the creature: fight it where it stands. Returns true if
    /// it is engaged with a creature (and so does nothing else this tick).
    /// A dwarf on their feet beside it becomes its target instead of one
    /// further off (the target code then strikes).
    pub(super) fn close_quarters(&mut self, c: &mut Creature) -> bool {
        // Its own quarry beside it: the usual blow (openings and all).
        if let Some(tid) = c.target {
            if self.dwarves.iter().any(|d| d.id == tid && d.alive && adjacent(d.pos, c.pos)) {
                return false;
            }
        }
        let wary = c.wary_until > self.tick;
        let fights_dwarves = c.hostile() && !wary && (c.siege || c.legendary() || c.target.is_some() || !super::wildlife::has_sense(c));
        if fights_dwarves {
            let beside = self.dwarves.iter().filter(|d| d.alive && !d.body.unconscious() && adjacent(d.pos, c.pos)).map(|d| d.id).min();
            if let Some(id) = beside {
                c.target = Some(id);
                c.path.clear();
                return false;
            }
        }
        let me = faction(c);
        if matches!(me, Faction::Fort | Faction::Neutral | Faction::Prey) {
            return false;
        }
        let pos = c.pos;
        let mine = c.foe.and_then(|f| self.creatures.get(&f)).filter(|o| o.alive && adjacent(o.pos, pos)).map(|o| o.id);
        let foe = mine.or_else(|| {
            self.creatures
                .values()
                .filter(|o| o.alive && adjacent(o.pos, pos) && !o.sneaking && hostile_to(me, faction(o)) && self.creature_active(o))
                .map(|o| o.id)
                .min()
        });
        let Some(oid) = foe else { return false };
        c.foe = Some(oid);
        c.path.clear();
        if c.attack_cd == 0 {
            c.attack_cd = c.species.info().attack_delay + c.body.limp();
            self.creature_maul(c, oid);
        }
        true
    }

    /// Standing beside its quarry on a tile another fighter shares: step to
    /// a free tile that's still beside the quarry.
    pub(super) fn spread_out(&mut self, c: &mut Creature, quarry: Pos) {
        if c.move_cd > 0 || !self.creatures.values().any(|o| o.alive && o.pos == c.pos) {
            return;
        }
        let spots: Vec<Pos> = DIRS8
            .iter()
            .map(|&(dx, dy)| quarry.offset(dx, dy, 0))
            .filter(|&q| q != c.pos && adjacent(q, c.pos) && self.world.in_bounds(q) && self.world.walkable(q))
            .collect();
        let mut nb = Vec::new();
        self.world.neighbors(c.pos, &mut nb, c.mover());
        if let Some(q) = spots.into_iter().find(|&q| nb.iter().any(|&(n, _)| n == q) && !self.someone_at(q, c.id)) {
            c.pos = q;
            c.path.clear();
            c.move_cd = c.species.info().move_delay;
        }
    }

    /// Whether the way it's going no longer brings it beside `to`.
    pub(super) fn needs_approach(&self, c: &Creature, to: Pos) -> bool {
        match c.path.first() {
            None => true,
            Some(&end) if !adjacent(end, to) => true,
            // Close now: aim for a free side rather than the quarry's own tile.
            Some(&end) if end == to => near(c.pos, to, 6),
            // Someone else has taken the spot it was making for.
            Some(&end) => c.path.len() <= 2 && self.someone_at(end, c.id),
        }
    }

    /// Set a path to a free tile beside `to` (close by), or straight at it
    /// (further off). False if there's no way.
    pub(super) fn close_in(&mut self, c: &mut Creature, to: Pos) -> bool {
        let mover = c.species.info().mover;
        if near(c.pos, to, 6) {
            let me = c.id;
            let taken: Vec<Pos> = self
                .creatures
                .values()
                .filter(|o| o.id != me && o.alive && near(o.pos, to, 1))
                .map(|o| o.pos)
                .chain(self.dwarves.iter().filter(|d| d.alive && !d.body.unconscious() && near(d.pos, to, 1)).map(|d| d.pos))
                .collect();
            let found = path::search(&self.world, c.pos, 600, mover, |q| q != to && adjacent(q, to) && !taken.contains(&q));
            if let Some((_, p)) = found {
                c.path = p.into_iter().rev().collect();
                return true;
            }
        }
        match path::astar(&self.world, c.pos, to, 8000, mover) {
            Some(p) => {
                c.path = p.into_iter().rev().collect();
                true
            }
            None => false,
        }
    }

    /// After the creature it's fighting or hunting.
    pub(super) fn pursue_foe(&mut self, c: &mut Creature) {
        let Some(oid) = c.foe else { return };
        let opos = self.creatures.get(&oid).filter(|o| o.alive && self.creature_active(o) && !o.sneaking).map(|o| o.pos);
        let Some(opos) = opos.filter(|&p| near(p, c.pos, 14)) else {
            c.foe = None;
            c.path.clear();
            return;
        };
        if c.move_cd == 0 && self.needs_approach(c, opos) && !self.close_in(c, opos) {
            c.foe = None;
            c.path.clear();
            c.think_cd = 90;
            return;
        }
        self.creature_step(c, c.species.info().move_delay);
    }

    /// From `creature_think`: a creature with no creature foe looks about
    /// for one it would fight. Armies, beasts and cave things only notice
    /// one close by; the dead and the monsters further; a predator with
    /// nobody to stalk now and then goes hunting the grazers.
    pub(super) fn seek_foe(&mut self, c: &mut Creature) {
        if let Some(f) = c.foe {
            if self.creatures.get(&f).map_or(false, |o| o.alive) {
                return;
            }
            c.foe = None;
        }
        if c.leaving || c.wary_until > self.tick {
            return;
        }
        let me = faction(c);
        let hunting = me == Faction::Predator && c.target.is_none() && c.stalk.is_none() && mix(c.pos, self.tick ^ c.id as u64) % 100 < self.knob("wild.hunt", 12.0) as u64;
        let r = match me {
            Faction::Dead | Faction::Monster => 8,
            Faction::Army(_) | Faction::Cave | Faction::Predator => 3,
            _ => return,
        };
        let pos = c.pos;
        let found = self
            .creatures
            .values()
            .filter(|o| o.id != c.id && o.alive && !o.sneaking && o.pos.z == pos.z)
            .filter(|o| {
                let f = faction(o);
                hostile_to(me, f) && near(o.pos, pos, if f == Faction::Prey { if hunting { 10 } else { 0 } } else { r })
            })
            .filter(|o| self.creature_active(o))
            .min_by_key(|o| (dist2(o.pos, pos), o.id))
            .map(|o| (o.id, o.pos));
        if let Some((id, p)) = found {
            if adjacent(p, pos) || self.can_reach(c, p, 400) {
                c.foe = Some(id);
                c.path.clear();
                self.count("melee.foes", 1);
            }
        }
    }

    /// One creature strikes another (`c` is out of the map, being ticked).
    pub(super) fn creature_maul(&mut self, c: &mut Creature, oid: u32) {
        let Some(o) = self.creatures.get(&oid) else {
            c.foe = None;
            return;
        };
        let att = self.creature_fighter(c);
        let def = self.creature_fighter(o);
        let (opos, osp, owner, seen) = (o.pos, o.species, owner_of_creature(o), a_or_name(o));
        let fights_back = hostile_to(faction(o), faction(c)) || o.siege || o.species.info().aggro > 0;
        let mut body = o.body.clone();
        let blow = self.strike(&att, &def, &mut body);
        self.count("melee.blows", 1);
        // Told only where the fortress can see it.
        let seen_here = self.world.t(opos).revealed && self.dwarves.iter().any(|d| d.alive && near(d.pos, opos, 25));
        if blow.notable && seen_here {
            if let Some(m) = &blow.msg {
                self.msg(m, (210, 180, 150));
            }
        }
        let knocked = self.aftermath(&blow, c.pos, opos, &owner, &seen, Some(osp), att.size >= 2.0);
        let dead = body.dead;
        let mut dropped = None;
        if let Some(o) = self.creatures.get_mut(&oid) {
            o.body = body;
            if let Some(q) = knocked {
                o.pos = q;
                o.path.clear();
                o.move_cd += 12;
                o.attack_cd += 12;
            }
            if blow.disarmed && !dead {
                dropped = o.weapon.take().map(|w| (w, o.pos));
            }
            // It turns on whoever struck it, if it's the fighting kind.
            if fights_back && !o.leaving && o.foe.is_none() {
                o.foe = Some(c.id);
            }
        }
        if let Some((w, at)) = dropped {
            self.drop_item(w, at);
        }
        if dead {
            c.foe = None;
            if let Some(o) = self.creatures.remove(&oid) {
                self.count("melee.kills", 1);
                if seen_here {
                    let what = if o.species == Species::Undead { "destroys" } else { "kills" };
                    self.msg(&format!("{} {} {}.", super::combat::cap(&c.title()), what, o.title()), (210, 180, 150));
                }
                self.creature_died(o);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::Part;

    /// Open ground on the surface with room around it, and no animals or
    /// visitors about.
    fn field(seed: u64) -> (Game, Pos) {
        let mut g = Game::new(seed);
        g.creatures.clear();
        let e = g.embark;
        let ok = |g: &Game, p: Pos| {
            (-3..=3).all(|dx| (-3..=3).all(|dy| {
                let q = p.offset(dx, dy, 0);
                g.world.in_bounds(q) && g.world.walkable(q) && g.world.t(q).water == 0 && g.world.t(q).building == 0 && q.z == g.world.surface_z(q.x, q.y)
            }))
        };
        for r in 4..40 {
            for dx in -r..=r {
                for dy in -r..=r {
                    let (x, y) = (e.x + dx, e.y + dy);
                    if x < 5 || y < 5 || x >= g.world.w - 5 || y >= g.world.h - 5 {
                        continue;
                    }
                    let p = Pos::new(x, y, g.world.surface_z(x, y));
                    if ok(&g, p) {
                        return (g, p);
                    }
                }
            }
        }
        panic!("no open field");
    }

    fn ctr(g: &Game, k: &str) -> u32 {
        g.annals.counters.get(k).copied().unwrap_or(0)
    }

    fn arm(g: &mut Game, id: u32, kind: ItemKind) {
        let p = g.creatures[&id].pos;
        let w = g.spawn_item(kind, Mat::Iron, p);
        g.items.get_mut(&w).unwrap().carried_by = Some(id);
        g.creatures.get_mut(&id).unwrap().weapon = Some(w);
    }

    /// Keep the dwarves well away (and out of the fight).
    fn send_dwarves_off(g: &mut Game, from: Pos) {
        for d in g.dwarves.iter_mut() {
            if near(d.pos, from, 12) {
                d.alive = false;
            }
        }
    }

    #[test]
    fn two_foes_side_by_side_keep_fighting_and_dont_wander() {
        let (mut g, p) = field(5);
        send_dwarves_off(&mut g, p);
        let gob = g.spawn_creature(Species::Goblin, p, u64::MAX);
        g.creatures.get_mut(&gob).unwrap().siege = true;
        let wolf = g.spawn_creature(Species::Bear, p.offset(1, 0, 0), u64::MAX);
        arm(&mut g, gob, ItemKind::Spear);
        let (mut blows, mut apart) = (0, 0);
        let mut last = ctr(&g, "melee.blows");
        for _ in 0..400 {
            g.step();
            let (Some(a), Some(b)) = (g.creatures.get(&gob), g.creatures.get(&wolf)) else { break };
            if a.leaving || b.leaving {
                break;
            }
            if !adjacent(a.pos, b.pos) {
                apart += 1;
            }
            let now = ctr(&g, "melee.blows");
            blows += (now - last) as i32;
            last = now;
        }
        assert!(blows >= 6, "only {} blows were struck", blows);
        assert!(apart <= 30, "they were apart for {} ticks", apart);
    }

    #[test]
    fn several_attackers_surround_one_target() {
        let (mut g, p) = field(7);
        send_dwarves_off(&mut g, p);
        // A dwarf stands alone in the field; four goblins come at her.
        let d = 0;
        g.dwarves[d].alive = true;
        g.dwarves[d].pos = p;
        g.dwarves[d].body = crate::body::Body::new(30.0);
        let mut ids = Vec::new();
        for i in 0..4 {
            let at = p.offset(-6 + i, -5, 0);
            let at = if g.world.walkable(at) { at } else { p.offset(-5, 0, 0) };
            let id = g.spawn_creature(Species::Goblin, at, u64::MAX);
            g.creatures.get_mut(&id).unwrap().siege = true;
            ids.push(id);
        }
        let mut best = 0;
        for _ in 0..300 {
            // Keep her in place and on her feet.
            let dd = &mut g.dwarves[d];
            dd.pos = p;
            dd.path.clear();
            dd.task = Task::Idle;
            dd.body = crate::body::Body::new(30.0);
            g.step();
            let around: HashSet<Pos> = ids.iter().filter_map(|id| g.creatures.get(id)).filter(|c| adjacent(c.pos, p) && c.pos != p).map(|c| c.pos).collect();
            best = best.max(around.len());
        }
        assert!(best >= 3, "at most {} goblins stood round her on separate tiles", best);
    }

    #[test]
    fn hostile_factions_fight_each_other_but_grazers_are_left_alone_by_armies() {
        use Faction::*;
        assert!(hostile_to(Army(Species::Goblin), Predator));
        assert!(hostile_to(Predator, Army(Species::Goblin)));
        assert!(hostile_to(Monster, Prey) && hostile_to(Dead, Army(Species::Human)));
        assert!(!hostile_to(Army(Species::Goblin), Prey));
        assert!(!hostile_to(Prey, Predator));
        assert!(!hostile_to(Army(Species::Goblin), Army(Species::Goblin)));
        assert!(!hostile_to(Predator, Fort) && !hostile_to(Monster, Neutral));
        // Goblins and a pack of wolves meet in the field: blood is drawn.
        let (mut g, p) = field(9);
        send_dwarves_off(&mut g, p);
        let gob = g.spawn_creature(Species::Goblin, p, u64::MAX);
        g.creatures.get_mut(&gob).unwrap().siege = true;
        arm(&mut g, gob, ItemKind::Axe);
        let w1 = g.spawn_creature(Species::Wolf, p.offset(3, 0, 0), u64::MAX);
        let w2 = g.spawn_creature(Species::Wolf, p.offset(3, 1, 0), u64::MAX);
        let deer = g.spawn_creature(Species::Deer, p.offset(-3, -3, 0), u64::MAX);
        let _ = (w1, w2);
        for _ in 0..600 {
            g.step();
        }
        assert!(ctr(&g, "melee.blows") > 0, "nobody fought");
        assert!(g.creatures.get(&gob).map_or(true, |c| !c.body.healed() || c.leaving) || ctr(&g, "melee.kills") > 0, "the goblin came through untouched");
        assert!(!g.gore.stains.is_empty(), "no blood was spilled");
        let _ = deer;
    }

    #[test]
    fn a_strong_edged_blow_can_sever_and_leaves_a_named_part() {
        let (mut g, p) = field(5);
        send_dwarves_off(&mut g, p);
        // A dwarf with an iron axe at a goblin, many times over.
        let gob = g.spawn_creature(Species::Goblin, p, u64::MAX);
        let mut d = g.dwarves.iter().find(|d| d.alive).unwrap().clone();
        let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, p);
        g.items.get_mut(&axe).unwrap().carried_by = Some(d.id);
        d.held = Some(axe);
        d.fight_xp = 3000;
        d.pos = p.offset(-1, 0, 0);
        let att = g.dwarf_fighter(&d);
        let def = g.creature_fighter(&g.creatures[&gob]);
        let mut severed = None;
        for _ in 0..400 {
            let mut body = crate::body::Body::new(0.9);
            for _ in 0..6 {
                let b = g.strike(&att, &def, &mut body);
                if b.severed.is_some() {
                    assert!(b.notable && b.blood >= 5);
                    severed = Some(b);
                    break;
                }
                if body.dead {
                    break;
                }
            }
            if severed.is_some() {
                break;
            }
        }
        let b = severed.expect("an iron axe never took a limb");
        let part = b.severed.unwrap();
        g.aftermath(&b, d.pos, p, "goblin", "a goblin", Some(Species::Goblin), false);
        let name = format!("goblin's {}", part.name());
        let item = g.items.values().find(|it| it.kind == ItemKind::BodyPart).expect("no part dropped");
        assert_eq!(item.name(), name);
        assert!(near(item.pos, p, 1));
        assert!(g.stain_at(p).is_some(), "no blood where the blow landed");
        assert!(ItemKind::BodyPart != ItemKind::Corpse && crate::items::StockFilter::Refuse.accepts(ItemKind::BodyPart));
        let _ = Part::Head;
    }

    #[test]
    fn the_reports_tell_of_a_severing_but_not_of_every_scratch() {
        let (mut g, p) = field(11);
        send_dwarves_off(&mut g, p);
        let gob = g.spawn_creature(Species::Goblin, p, u64::MAX);
        g.creatures.get_mut(&gob).unwrap().siege = true;
        // A soldier with an axe beside it, in a long fight (both made tough).
        let di = 0;
        let did = g.dwarves[di].id;
        {
            let d = &mut g.dwarves[di];
            d.alive = true;
            d.pos = p.offset(1, 0, 0);
            d.soldier = true;
            d.fight_xp = 1600;
        }
        let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, p);
        g.items.get_mut(&axe).unwrap().carried_by = Some(did);
        g.dwarves[di].held = Some(axe);
        let before = g.log.len();
        let mut swings = 0;
        for _ in 0..200 {
            if !g.creatures.contains_key(&gob) {
                break;
            }
            let mut d = std::mem::take(&mut g.dwarves[di]);
            d.attack_cd = 0;
            g.dwarf_attack(&mut d, gob);
            g.dwarves[di] = d;
            swings += 1;
            if let Some(c) = g.creatures.get_mut(&gob) {
                if c.body.blood < 40 {
                    c.body.blood = 80;
                }
            }
        }
        let lines: Vec<&str> = g.log[before..].iter().map(|l| l.text.as_str()).collect();
        let told = lines.iter().filter(|l| l.contains(" in the ")).count();
        assert!(told > 0, "nothing told of {} swings", swings);
        assert!(told * 2 < swings, "{} of {} swings told: {:?}", told, swings, lines);
        assert!(lines.iter().all(|l| !l.contains("deflects") && !l.contains("blocks")), "scratches told: {:?}", lines);
        if ctr(&g, "gore.severed") > 0 {
            assert!(lines.iter().any(|l| l.contains("severing") || l.contains("cutting off")), "a severing went untold: {:?}", lines);
        }
    }
}
