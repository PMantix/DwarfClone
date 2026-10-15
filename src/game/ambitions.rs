//! The overseer's ambitions: what it has its heart set on for the next
//! season or so. Each style leans towards some more than others, and the
//! fortress's troubles push it towards what's needed.

use super::overseer::{Overseer, RoomUse, Style};
use super::*;
use crate::buildings::Recipe;
use crate::game::expeditions::Goal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ambition {
    /// Fields, fishing, hunting, brewing and cooking.
    Provision,
    /// Traps, gates and a walled yard.
    Fortify,
    /// More soldiers, more weapons and armour.
    Muster,
    /// Crafts, gems, ore and trade.
    Prosper,
    /// Down the shaft, into the deep places.
    Delve,
    /// New halls and levels.
    Expand,
    /// Taverns, temples, ale and fine engravings.
    Revel,
    /// Expeditions out into the world.
    Venture,
}

impl Ambition {
    pub const ALL: [Ambition; 8] = [
        Ambition::Provision,
        Ambition::Fortify,
        Ambition::Muster,
        Ambition::Prosper,
        Ambition::Delve,
        Ambition::Expand,
        Ambition::Revel,
        Ambition::Venture,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Ambition::Provision => "Provision",
            Ambition::Fortify => "Fortify",
            Ambition::Muster => "Muster",
            Ambition::Prosper => "Prosper",
            Ambition::Delve => "Delve",
            Ambition::Expand => "Expand",
            Ambition::Revel => "Revel",
            Ambition::Venture => "Venture",
        }
    }

    pub fn announce(self) -> &'static str {
        match self {
            Ambition::Provision => "The overseer frets over the larders: more fields, fishing and hunting.",
            Ambition::Fortify => "The overseer looks to the walls: traps, gates and a walled yard.",
            Ambition::Muster => "The overseer calls the fortress to arms: more soldiers, more weapons.",
            Ambition::Prosper => "The overseer dreams of riches: crafts, gems and ore for the caravans.",
            Ambition::Delve => "The overseer turns to the deep places. The shaft goes down.",
            Ambition::Expand => "The overseer wants room to grow: new halls are dug.",
            Ambition::Revel => "The overseer decides the dwarves deserve some cheer: taverns, temples and ale.",
            Ambition::Venture => "The overseer grows restless: the squad will march out to seek glory.",
        }
    }

    /// How much a style is drawn to each ambition.
    fn weight(self, style: Style) -> f64 {
        use Ambition::*;
        match (style, self) {
            (Style::Steady, Delve | Revel | Venture) => 1.0,
            (Style::Steady, _) => 2.0,
            (Style::Builder, Prosper | Expand | Revel) => 3.0,
            (Style::Builder, Provision) => 2.0,
            (Style::Builder, Venture) => 0.5,
            (Style::Builder, _) => 1.0,
            (Style::Delver, Delve) => 4.0,
            (Style::Delver, Prosper) => 3.0,
            (Style::Delver, Provision) => 2.0,
            (Style::Delver, _) => 1.0,
            (Style::Warlord, Muster | Venture) => 4.0,
            (Style::Warlord, Fortify | Provision) => 2.0,
            (Style::Warlord, _) => 1.0,
            (Style::Reckless, Fortify | Muster) => 0.0,
            (Style::Reckless, Delve | Revel) => 3.0,
            (Style::Reckless, Provision) => 1.0,
            (Style::Reckless, _) => 2.0,
        }
    }
}

impl Game {
    fn per_head(&self, kinds: &[ItemKind]) -> f64 {
        let n: usize = kinds.iter().map(|&k| self.stock_of(k)).sum();
        n as f64 / self.alive_count().max(1) as f64
    }

    fn food_per_head(&self) -> f64 {
        self.per_head(&[ItemKind::Meal, ItemKind::Plant, ItemKind::Meat, ItemKind::Fish])
    }

    /// Daily: keep at the current ambition, or settle on a new one.
    pub(super) fn ov_ambition(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let since = tick.saturating_sub(o.ambition_since);
        let finished = o.ambition.map_or(true, |a| since >= SEASON || (since >= DAY * 10 && self.ambition_met(o, a)));
        if finished {
            self.choose_ambition(o);
        }
        if let Some(a) = o.ambition {
            self.pursue(o, a);
        }
    }

    /// Whether the current ambition has got what it wanted.
    fn ambition_met(&self, o: &Overseer, a: Ambition) -> bool {
        match a {
            Ambition::Provision => self.food_per_head() >= 8.0 && self.per_head(&[ItemKind::Drink]) >= 5.0,
            Ambition::Fortify => o.did("stonetraps") && o.did("yard"),
            Ambition::Muster => {
                let soldiers = self.dwarves.iter().filter(|d| d.alive && d.soldier).count();
                soldiers >= self.muster_target(o)
            }
            Ambition::Prosper => self.wealth() >= o.ambition_mark.saturating_mul(13) / 10,
            Ambition::Delve => o.done.iter().filter(|d| d.starts_with("shaft")).count() as u32 >= o.ambition_mark + 2,
            Ambition::Expand | Ambition::Revel => o.wants.is_empty() && o.rooms.iter().all(|r| r.stage == 1),
            Ambition::Venture => o.did(&format!("ventured{}", o.ambition_since)) && self.expedition.is_none(),
        }
    }

    fn choose_ambition(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let besieged = tick < o.last_siege + SEASON && o.last_siege > 0;
        let beds = self.buildings.values().filter(|b| b.kind == BuildingKind::Bed).count();
        let weights: Vec<(Ambition, f64)> = Ambition::ALL
            .iter()
            .map(|&a| {
                let mut w = a.weight(o.style);
                match a {
                    Ambition::Provision if self.food_per_head() < 4.0 || self.per_head(&[ItemKind::Drink]) < 3.0 => w *= 5.0,
                    Ambition::Fortify | Ambition::Muster if besieged => w *= 3.0,
                    Ambition::Expand if beds < self.alive_count() => w *= 4.0,
                    Ambition::Venture if self.squads.is_empty() || tick < YEAR / 2 => w = 0.0,
                    _ => {}
                }
                if o.ambition == Some(a) {
                    w *= 0.2;
                }
                (a, w)
            })
            .collect();
        let total: f64 = weights.iter().map(|w| w.1).sum();
        if total <= 0.0 {
            return;
        }
        let mut r = self.rng.range(0, 10_000) as f64 / 10_000.0 * total;
        let mut pick = weights[0].0;
        for (a, w) in &weights {
            if r < *w {
                pick = *a;
                break;
            }
            r -= w;
        }
        o.ambition = Some(pick);
        o.ambition_since = tick;
        o.ambition_mark = match pick {
            Ambition::Prosper => self.wealth(),
            Ambition::Delve => o.done.iter().filter(|d| d.starts_with("shaft")).count() as u32,
            _ => 0,
        };
        o.ambitions.push((tick, pick));
        self.msg(pick.announce(), (150, 210, 255));
        Self::note_ov(o, tick, format!("Set its heart on: {}.", pick.name()));
        // What some ambitions want dug.
        let has = |o: &Overseer, k: RoomUse| o.rooms.iter().filter(|r| r.kind == k).count();
        match pick {
            Ambition::Expand => {
                o.wants.extend([RoomUse::Dormitory, RoomUse::Stockpile, RoomUse::Dining]);
                if self.alive_count() >= 20 {
                    o.wants.push(RoomUse::Workshops);
                }
            }
            Ambition::Revel => {
                if has(o, RoomUse::Tavern) < 1 + self.alive_count() / 25 {
                    o.wants.push(RoomUse::Tavern);
                }
                if has(o, RoomUse::Temple) == 0 {
                    o.wants.push(RoomUse::Temple);
                }
            }
            Ambition::Muster if has(o, RoomUse::Barracks) == 0 => o.wants.push(RoomUse::Barracks),
            _ => {}
        }
    }

    pub(super) fn muster_target(&self, o: &Overseer) -> usize {
        let adults = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        ((adults as f64 * 0.35).ceil() as usize).max(4).min(adults.saturating_sub(4)).max(if o.style == Style::Reckless { 0 } else { 2 })
    }

    /// A day's work towards the ambition.
    fn pursue(&mut self, o: &mut Overseer, a: Ambition) {
        let tick = self.tick;
        let e = self.embark;
        let base = o.base.unwrap();
        let top = o.top.unwrap();
        match a {
            Ambition::Provision => {
                let k = o.done.iter().filter(|d| d.starts_with("farm")).count() as i32;
                let (dx, dy) = [(-20, -8), (12, -8), (-20, 6), (12, 6), (-8, -18), (2, -18), (-8, 14), (2, 14)][(k % 8) as usize];
                if k < 12 && self.add_zone(e.offset(dx, dy, -1), e.offset(dx + 7, dy + 5, 1), ZoneKind::Farm) > 0 {
                    Self::note_ov(o, tick, "Sowed new fields.".into());
                }
                o.done.push(format!("farm{}", k));
                let fishers = self.dwarves.iter().filter(|d| d.alive && d.has(Labor::Fishing)).count();
                if fishers < 2 {
                    if let Some(id) = self.dwarves.iter().filter(|d| d.alive && !d.is_child() && !d.soldier && !d.has(Labor::Fishing)).max_by_key(|d| d.skills[Labor::Fishing.idx()]).map(|d| d.id) {
                        self.set_labor(id, Labor::Fishing, true);
                    }
                }
                if self.buildings.values().all(|b| b.kind != BuildingKind::Butcher) {
                    if let Some(p) = self.ov_spot(BuildingKind::Butcher, e.offset(6, 6, 0), 16) {
                        self.place_building(BuildingKind::Butcher, p);
                    }
                }
            }
            Ambition::Fortify => {
                // Stone-fall traps further down the corridor (boulders are plenty).
                if !o.did("stonetraps") && self.stock_of(ItemKind::Mechanism) >= 2 && self.stock_of(ItemKind::Boulder) >= 2 {
                    let mut n = 0;
                    for dx in 10..14 {
                        for dy in 0..2 {
                            if self.place_building(BuildingKind::StoneTrap, base.offset(dx, dy, 0)) {
                                n += 1;
                            }
                        }
                    }
                    if n > 0 {
                        o.done.push("stonetraps".into());
                        Self::note_ov(o, tick, format!("Set {} stone-fall traps deeper in the corridor.", n));
                    }
                }
                // A walled yard round the top of the stair, with a door in it.
                if !o.did("yard") {
                    let r = 3;
                    let mut n = 0;
                    for (a, b) in [
                        (top.offset(-r, -r, 0), top.offset(r, -r, 0)),
                        (top.offset(-r, r, 0), top.offset(r, r, 0)),
                        (top.offset(r, -r, 0), top.offset(r, r, 0)),
                        (top.offset(-r, -r, 0), top.offset(-r, -1, 0)),
                        (top.offset(-r, 1, 0), top.offset(-r, r, 0)),
                    ] {
                        n += self.plan_constructions(a, b, super::ConstructKind::Wall, super::BuildMat::Any);
                    }
                    self.place_building(BuildingKind::Door, top.offset(-r, 0, 0));
                    o.done.push("yard".into());
                    Self::note_ov(o, tick, format!("Planned a walled yard round the stair ({} wall sections) with a gate.", n));
                }
            }
            Ambition::Muster => {
                if self.squads.len() < 2 && self.dwarves.iter().filter(|d| d.alive && d.soldier).count() >= 6 {
                    self.new_squad();
                    let name = self.squads.last().unwrap().name.clone();
                    Self::note_ov(o, tick, format!("Raised a second squad, {}.", name));
                }
            }
            Ambition::Prosper => {
                self.ov_mine(o, 30, 30);
                if self.wealth() >= 30_000 && o.style != Style::Reckless && !o.did("monument") {
                    if let Some(p) = self.ov_spot(BuildingKind::Monument, e.offset(0, 6, 0), 16) {
                        self.place_building(BuildingKind::Monument, p);
                        o.done.push("monument".into());
                        Self::note_ov(o, tick, "Began a great monument.".into());
                    }
                }
                self.ov_detail(o, true);
            }
            Ambition::Delve => self.ov_shaft(o),
            Ambition::Expand => {}
            Ambition::Revel => self.ov_detail(o, true),
            Ambition::Venture => {
                if self.ov_venture(o, 0.5, true) {
                    o.done.push(format!("ventured{}", o.ambition_since));
                }
            }
        }
    }

    /// Work orders the ambition adds or raises.
    pub(super) fn ambition_orders(&self, o: &Overseer, orders: &mut Vec<(Recipe, u32)>) {
        let pop = self.alive_count() as u32;
        let raise = |orders: &mut Vec<(Recipe, u32)>, r: Recipe, keep: u32| match orders.iter_mut().find(|x| x.0 == r) {
            Some(x) => x.1 = x.1.max(keep),
            None => orders.push((r, keep)),
        };
        match o.ambition {
            Some(Ambition::Provision) => {
                raise(orders, Recipe::Brew, 40 + pop * 3);
                raise(orders, Recipe::Cook, 20 + pop * 2);
                raise(orders, Recipe::Mill, 10);
            }
            Some(Ambition::Fortify) => {
                raise(orders, Recipe::Mechanism, 10);
                raise(orders, Recipe::WoodCage, 8);
                raise(orders, Recipe::Blocks, 30);
            }
            Some(Ambition::Muster) => {
                raise(orders, Recipe::WoodCrossbow, 6);
                raise(orders, Recipe::Bolts, 16);
                raise(orders, Recipe::LeatherArmor, 4);
                raise(orders, Recipe::LeatherHelm, 4);
                raise(orders, Recipe::Tan, 4);
                raise(orders, Recipe::ForgeSpear, 3);
                raise(orders, Recipe::ForgeHelm, 4);
                raise(orders, Recipe::ForgeBreastplate, 3);
                raise(orders, Recipe::ForgeShield, 3);
                raise(orders, Recipe::SmeltOre, 10);
            }
            Some(Ambition::Prosper) => {
                raise(orders, Recipe::StoneCraft, 30);
                raise(orders, Recipe::CutGem, 10);
                raise(orders, Recipe::BoneCraft, 10);
                raise(orders, Recipe::LeatherCraft, 6);
                raise(orders, Recipe::StoneTable, 4);
                raise(orders, Recipe::StoneChair, 4);
                raise(orders, Recipe::SmeltOre, 10);
            }
            Some(Ambition::Revel) => raise(orders, Recipe::Brew, 60 + pop * 3),
            Some(Ambition::Expand) => {
                raise(orders, Recipe::WoodBed, 6);
                raise(orders, Recipe::WoodTable, 4);
                raise(orders, Recipe::WoodChair, 4);
                raise(orders, Recipe::WoodDoor, 4);
            }
            _ => {}
        }
    }

    /// Mine ore and gems that have come to light near the halls: up to
    /// `max` tiles, looking `reach` tiles out.
    pub(super) fn ov_mine(&mut self, o: &Overseer, max: usize, reach: i32) {
        let base = o.base.unwrap();
        let mut n = 0;
        for dz in -8..=0 {
            for dy in -reach..=reach {
                for dx in -reach..=(reach + 8 * o.slots) {
                    let p = base.offset(dx, dy, dz);
                    let Some(t) = self.world.get(p) else { continue };
                    // Only what a miner can stand beside: next to ground we've dug ourselves.
                    let reachable = || DIRS8.iter().any(|&(ex, ey)| self.world.get(p.offset(ex, ey, 0)).map_or(false, |q| q.dug && !q.kind.solid() && q.water <= 3));
                    if t.revealed && t.kind == TileKind::Wall && t.desig == Desig::None && (t.mat.is_ore() || t.mat.is_gem()) && t.built_from.is_none() && reachable() {
                        self.designate(p, p, Desig::Dig);
                        n += 1;
                        if n >= max {
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Sink the exploring shaft another few levels, with a tunnel at the bottom.
    pub(super) fn ov_shaft(&mut self, o: &mut Overseer) {
        let base = o.base.unwrap();
        let shaft = base.offset(-1, 0, 0);
        let deep = (1..=base.z).rev().take_while(|&z| self.world.kind(Pos::new(shaft.x, shaft.y, z)).map_or(false, |k| matches!(k, TileKind::StairUD | TileKind::StairDown))).last().unwrap_or(base.z);
        let digging = self.designations.iter().filter(|p| p.x == shaft.x && p.y == shaft.y).count();
        if deep > 3 && digging == 0 && self.designations.len() < 80 {
            let to = Pos::new(shaft.x, shaft.y, (deep - 4).max(2));
            self.designate(Pos::new(shaft.x, shaft.y, deep), to, Desig::StairUD);
            self.designate(to.offset(-12, 0, 0), to.offset(12, 0, 0), Desig::Dig);
            self.designate(to.offset(0, -12, 0), to.offset(0, 12, 0), Desig::Dig);
            let key = format!("shaft{}", to.z);
            if !o.did(&key) {
                o.done.push(key);
                Self::note_ov(o, self.tick, format!("Sank the shaft to level {}.", to.z));
            }
        }
        self.ov_mine(o, 20, 14);
    }

    /// Smooth, then engrave, the halls folk gather in.
    pub(super) fn ov_detail(&mut self, o: &mut Overseer, engrave: bool) {
        let detailing = self.designations.iter().filter(|p| matches!(self.world.t(**p).desig, Desig::Smooth | Desig::Engrave)).count();
        if detailing >= 40 {
            return;
        }
        let halls: Vec<_> = o.rooms.iter().filter(|r| r.stage == 1 && matches!(r.kind, RoomUse::Dining | RoomUse::Tavern | RoomUse::Temple | RoomUse::Meeting)).cloned().collect();
        for r in halls {
            let smooth = format!("engrave{}", r.a.x * 1000 + r.a.y + r.a.z * 1_000_000);
            if !o.did(&smooth) {
                self.designate(r.a.offset(-1, -1, 0), r.b.offset(1, 1, 0), Desig::Smooth);
                o.done.push(smooth);
                return;
            }
            let carve = format!("carve{}", r.a.x * 1000 + r.a.y + r.a.z * 1_000_000);
            if engrave && !o.did(&carve) && self.tick > YEAR / 3 {
                self.designate(r.a.offset(-1, -1, 0), r.b.offset(1, 1, 0), Desig::Engrave);
                o.done.push(carve);
                return;
            }
        }
    }

    /// Send the first squad on the most worthwhile expedition it can likely
    /// win. True if they went.
    pub(super) fn ov_venture(&mut self, o: &mut Overseer, bold: f64, certain: bool) -> bool {
        if self.expedition.is_some() || self.alert || self.squads.is_empty() || o.style == Style::Reckless && !certain {
            return false;
        }
        let members = self.squads[0].members.clone();
        let strength = self.squad_strength(&members);
        if members.len() < 3 || strength < 8 || self.tick < YEAR / 2 {
            return false;
        }
        let pick = self.destinations().into_iter().filter(|d| strength as f64 / (strength as f64 + d.danger as f64 * 3.0) >= bold).max_by_key(|d| {
            let worth = match d.goal {
                Goal::Beast(_) => 3,
                Goal::Raid => 2,
                Goal::Ruin => 1,
            };
            (worth, -d.dist)
        });
        let Some(d) = pick else { return false };
        if !certain && !self.rng.chance(0.2) {
            return false;
        }
        let name = d.name.clone();
        if self.send_expedition(0, d).is_ok() {
            Self::note_ov(o, self.tick, format!("Sent the squad to {}.", name));
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overseer_moves_from_ambition_to_ambition() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Builder));
        for _ in 0..(SEASON * 3) {
            g.step();
            if g.alive_count() == 0 {
                break;
            }
        }
        let o = g.overseer.as_ref().unwrap();
        assert!(o.ambitions.len() >= 2, "only {:?}", o.ambitions);
        assert!(o.ambition.is_some());
        assert!(g.log.iter().any(|l| l.text.starts_with("The overseer")), "ambitions should be announced");
        // Never the same twice running is likely, but not forced; any two in a row
        // should at least both be real ambitions.
        assert!(o.ambitions.iter().all(|a| Ambition::ALL.contains(&a.1)));
    }

    #[test]
    fn rooms_go_on_to_deeper_levels() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        // Run long enough to dig in, then ask for a lot of rooms.
        for _ in 0..DAY * 3 {
            g.step();
        }
        let o = g.overseer.as_ref().unwrap().clone();
        let per = g.ov_per_level(&o) as usize;
        let (_, _, level) = g.ov_room_box(&o, per * 2).expect("a second level");
        assert_eq!(level, 1);
        let (a, _, _) = g.ov_room_box(&o, per * 2).unwrap();
        assert_eq!(a.z, o.base.unwrap().z - 2);
        assert!(a.x + 10 < g.world.w, "rooms stay on the map");
    }
}
