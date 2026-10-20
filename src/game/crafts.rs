//! Crafts that remember (O03 step 5): the skulls of great foes kept as
//! trophies and set up for all to see, weapons that earn a name with their
//! kills, makers' marks and inscriptions on fine work, and gems set into
//! furniture by the jeweler.

use super::memory::MemoryKind;
use super::names::{self, NameTheme};
use super::*;
use super::overseer::{Overseer, Room, RoomUse};
use crate::creature::{Creature, Species};
use crate::items::{ARTIFACT, NAMED_AT};

/// How far a dwarf can stand from a trophy and still stop to admire it.
const ADMIRE_RANGE: i32 = 6;

impl Game {
    // ------------------------------------------------------------------
    // Trophies of war
    // ------------------------------------------------------------------

    /// A foe worth remembering died in the fort (`creature_died`): a dragon,
    /// a named beast, a horror from the deep or the general of a siege. Its
    /// skull is kept as a trophy.
    pub(super) fn take_trophy(&mut self, c: &Creature) {
        let general = c.general.is_some();
        if !(c.breathes_fire || c.figure.is_some() || general || c.species == Species::CaveHorror) {
            return;
        }
        let label = match (&c.desc, general) {
            (Some(desc), true) => format!("skull of {}, {}", c.title(), desc),
            _ => format!("skull of {}", c.title()),
        };
        let id = self.spawn_item(ItemKind::Trophy, Mat::Bone, c.pos);
        if let Some(it) = self.items.get_mut(&id) {
            it.quality = 5;
            it.label = Some(label.clone());
        }
        self.msg(&format!("The {} is taken as a trophy of war.", label), (240, 225, 170));
        self.record(MemoryKind::Feat, &[], Some(c.pos), format!("The {} was taken as a trophy.", label), 4);
    }

    /// A trophy is about to be set up as a display (`Task::Construct`): the
    /// display takes its name.
    pub(super) fn trophy_set_up(&mut self, item: u32, bld: u32) {
        let Some(label) = self.items.get(&item).filter(|it| it.kind == ItemKind::Trophy).and_then(|it| it.label.clone()) else { return };
        let Some(b) = self.buildings.get_mut(&bld) else { return };
        if b.kind != BuildingKind::Display {
            return;
        }
        b.name = Some(label.clone());
        let at = b.pos;
        let fort = self.fortress_name.clone();
        self.msg(&format!("The {} has been set up for all to see.", label), (240, 225, 170));
        self.legend(format!("The {} was set up in the halls of {}.", label, fort));
        self.spotlight("A trophy of war", &format!("The {} is set up in the halls, for all to see.", label), Some(at), (240, 225, 170));
    }

    /// Daily: dwarves who pass a trophy on display stop to admire it.
    pub(super) fn trophies_daily(&mut self) {
        let shown: Vec<(Pos, String)> = self
            .buildings
            .values()
            .filter(|b| b.kind == BuildingKind::Display && b.built)
            .filter_map(|b| b.name.clone().map(|n| (b.pos, n)))
            .collect();
        let tick = self.tick;
        for (at, name) in shown {
            let text = format!("admired the {}", name);
            for d in self.dwarves.iter_mut().filter(|d| d.alive && d.pos.z == at.z && (d.pos.x - at.x).abs() <= ADMIRE_RANGE && (d.pos.y - at.y).abs() <= ADMIRE_RANGE) {
                if !d.thoughts.iter().any(|t| t.text == text && tick < t.tick + DAY * 3) {
                    d.think(&text, -6, tick);
                }
            }
        }
    }

    /// A people's next siege, if we hold a trophy taken from them: what the
    /// herald adds ("They have come for the skull of their general.").
    pub(super) fn trophy_vengeance(&self, civ_name: &str) -> String {
        let of = format!("general of the {}", civ_name);
        let held = self.items.values().filter(|it| it.kind == ItemKind::Trophy).filter_map(|it| it.label.clone());
        let shown = self.buildings.values().filter(|b| b.kind == BuildingKind::Display).filter_map(|b| b.name.clone());
        match held.chain(shown).find(|l| l.ends_with(&of)) {
            Some(l) => format!(" They have come for the {}.", l.split(',').next().unwrap_or(&l)),
            None => String::new(),
        }
    }

    // ------------------------------------------------------------------
    // Named weapons
    // ------------------------------------------------------------------

    /// `d` killed `c` with the weapon in hand (`credit_kill`): the weapon
    /// counts it, and at its fifth kill it earns a name.
    pub(super) fn weapon_blooded(&mut self, d: &mut Dwarf, c: &Creature) {
        let Some(w) = d.held else { return };
        let Some(it) = self.items.get_mut(&w) else { return };
        if it.kind.weapon_damage().is_none() {
            return;
        }
        it.kills = it.kills.saturating_add(1);
        let (kills, artifact) = (it.kills, it.quality >= ARTIFACT);
        let tick = self.tick;
        let who = d.called();
        let fort = self.fortress_name.clone();
        if kills == NAMED_AT && !artifact && it.label.is_none() {
            let plain = it.name();
            let name = names::coin(names::hash(&[super::stories::hash(&fort, 0), w as u64]), NameTheme::Weapon);
            it.label = Some(name.clone());
            d.think(&format!("is attached to {}", name), -15, tick);
            let text = format!("{}'s {} has drunk the blood of five foes, and is called {} now.", who, plain, name);
            self.msg(&text, (255, 215, 60));
            self.legend(format!("{} of {} slew {}, the fifth foe to fall to a {}, and named it {}.", who, fort, c.title(), plain, name));
            self.record(MemoryKind::Feat, &[d.id], Some(d.pos), format!("{} named {} {} after its fifth kill.", who, plain, name), 5);
            self.spotlight("A weapon is named", &text, Some(d.pos), (255, 215, 60));
            return;
        }
        let Some(name) = it.label.clone().filter(|_| kills >= NAMED_AT || artifact) else { return };
        // A named blade's great deeds go into the legends with it.
        if c.breathes_fire || c.figure.is_some() || c.general.is_some() {
            self.legend(format!("{} slew {} with {}.", who, c.title(), name));
        } else if kills == 25 || kills == 50 {
            self.legend(format!("{} has slain {} foes in the hands of the dwarves of {}.", name, kills, fort));
            self.msg(&format!("{} has slain its {}th foe.", name, kills), (255, 215, 60));
        }
    }

    // ------------------------------------------------------------------
    // Makers' marks and the jeweler
    // ------------------------------------------------------------------

    /// Something has just been made at a workshop: its maker's mark, and a
    /// gem set into a piece keeps the piece's quality and maker.
    pub(super) fn crafts_made(&mut self, d: &Dwarf, recipe: Recipe, used: &[Item], made: &[u32]) {
        for &id in made {
            let Some(it) = self.items.get_mut(&id) else { continue };
            if recipe == Recipe::Encrust {
                if let (Some(gem), Some(piece)) = (used.first(), used.get(1)) {
                    it.decor = Some(gem.mat);
                    it.quality = piece.quality;
                    it.maker = piece.maker;
                    it.kills = piece.kills;
                }
            } else if it.kind.base_value() >= 10 && !it.kind.is_food() {
                it.maker = Some(d.id);
            }
        }
    }

    /// What the look panel says of a piece's making and history.
    pub fn craft_lines(&self, id: u32) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(b) = self.buildings.get(&id) {
            if b.kind == BuildingKind::Display {
                if let Some(n) = &b.name {
                    v.push(format!("It shows the {}.", n));
                }
            }
            return v;
        }
        let Some(it) = self.items.get(&id) else { return v };
        if it.kind.weapon_damage().is_some() && it.kills > 0 {
            let foes = if it.kills == 1 { "one foe".to_string() } else { format!("{} foes", it.kills) };
            match &it.label {
                Some(name) if it.kills >= NAMED_AT && it.quality < ARTIFACT => v.push(format!("It has slain {}, and was named {} at its fifth kill.", foes, name)),
                _ => v.push(format!("It has slain {}.", foes)),
            }
        }
        if let Some(g) = it.decor {
            v.push(format!("It is set with {}.", g.name()));
        }
        let maker = it.maker.and_then(|m| self.dwarves.iter().find(|d| d.id == m)).map(|d| d.name.clone());
        if let Some(m) = maker.filter(|_| it.quality >= 3) {
            v.push(format!("It bears the mark of its maker, {}.", m));
            if it.quality >= 4 && it.quality < ARTIFACT {
                v.push(self.inscription(it, &m));
            }
        }
        v
    }

    /// The words cut into exceptional work: the same piece always says the
    /// same thing.
    fn inscription(&self, it: &Item, maker: &str) -> String {
        let first = maker.split(' ').next().unwrap_or(maker);
        let fort = &self.fortress_name;
        let h = names::hash(&[it.id as u64, 7]);
        let forms = [
            "Inscribed on it: \"{m} made me, at {f}.\"",
            "Inscribed on it: \"Stone remembers. So shall {f}.\"",
            "Inscribed on it: \"By the hand of {m}, for the halls of {f}.\"",
            "Inscribed on it: \"Let no one say {m} worked idly.\"",
            "Inscribed on it, in tiny runes: \"{m} was here.\"",
        ];
        super::stories::pick(h, 1, &forms).replace("{m}", first).replace("{f}", fort)
    }

    // ------------------------------------------------------------------
    // The overseer
    // ------------------------------------------------------------------

    /// Daily (`ov_daily_hooks`): trophies go on display in the great hall,
    /// and a rich fort has its jeweler set gems into its furniture.
    pub(super) fn ov_crafts(&mut self, o: &mut Overseer) {
        if o.base.is_none() {
            return;
        }
        self.ov_displays(o);
        self.ov_jeweler(o);
    }

    fn ov_displays(&mut self, o: &Overseer) {
        let trophies = self.items.values().filter(|it| it.kind == ItemKind::Trophy && !it.forbidden && it.carried_by.map_or(true, |c| !self.creatures.contains_key(&c))).count();
        let waiting = self.buildings.values().filter(|b| b.kind == BuildingKind::Display && !b.built).count();
        if trophies <= waiting {
            return;
        }
        // The great hall first, then any hall where the fort gathers.
        let mut halls: Vec<&Room> = o.rooms.iter().filter(|r| r.stage == 1 && matches!(r.kind, RoomUse::Dining | RoomUse::Grand | RoomUse::Meeting | RoomUse::Tavern)).collect();
        halls.sort_by_key(|r| (o.interior.great != Some(r.a), r.kind != RoomUse::Grand, r.since));
        let halls: Vec<Room> = halls.into_iter().cloned().collect();
        for r in halls {
            let spot = self.wall_spots(&r).into_iter().find(|&p| self.world.t(p).building == 0 && self.can_place(BuildingKind::Display, p));
            if let Some(p) = spot {
                if self.place_building(BuildingKind::Display, p) {
                    return;
                }
            }
        }
    }

    fn ov_jeweler(&mut self, o: &Overseer) {
        let keen = o.ambition == Some(super::ambitions::Ambition::Prosper);
        if o.interior.tier < 3 && !(keen && o.interior.tier >= 2) {
            return;
        }
        if self.queued(Recipe::Encrust) > 0 {
            return;
        }
        let gems = self.items.values().filter(|it| it.free() && it.kind == ItemKind::Gem).count();
        let pieces = self.items.values().filter(|it| it.free() && crate::buildings::Input::Furniture.accepts(it)).count();
        // Keep a few gems back for trade.
        if gems < 2 || pieces == 0 {
            return;
        }
        let shop = self.buildings.values().filter(|b| b.kind == BuildingKind::Craftsdwarf && b.built && b.orders.len() < 3).map(|b| b.id).min();
        if let Some(b) = shop.and_then(|id| self.buildings.get_mut(&id)) {
            b.orders.push(Order { recipe: Recipe::Encrust, repeat: false });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::Species;

    #[test]
    fn a_weapon_that_kills_enough_gets_a_name() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let mut d = std::mem::take(&mut g.dwarves[0]);
        let sword = g.spawn_item(ItemKind::Sword, Mat::Iron, d.pos);
        g.items.get_mut(&sword).unwrap().carried_by = Some(d.id);
        d.held = Some(sword);
        let id = g.spawn_creature(Species::Wolf, d.pos, DAY);
        let c = g.creatures.remove(&id).unwrap();
        for _ in 0..NAMED_AT - 1 {
            g.credit_kill(&mut d, &c);
        }
        assert!(g.items[&sword].label.is_none(), "not yet");
        g.credit_kill(&mut d, &c);
        let it = &g.items[&sword];
        let name = it.label.clone().expect("the sword was never named");
        assert_eq!(it.kills, NAMED_AT);
        assert!(it.name().starts_with(&name), "{}", it.name());
        assert!(d.thoughts.iter().any(|t| t.text == format!("is attached to {}", name)));
        assert!(g.overworld.events.iter().any(|e| e.text.contains(&name)), "no legend");
        assert!(g.craft_lines(sword).iter().any(|l| l.contains("slain 5 foes")));
        // The same sword in the same fort always earns the same name.
        let again = names::coin(names::hash(&[super::super::stories::hash(&g.fortress_name, 0), sword as u64]), NameTheme::Weapon);
        assert_eq!(name, again);
        g.dwarves[0] = d;
    }

    #[test]
    fn a_trophy_goes_on_display_and_is_admired() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        // A siege's general falls in the fort.
        let id = g.spawn_creature(Species::Goblin, e, DAY);
        let mut c = g.creatures.remove(&id).unwrap();
        c.label = Some("Snodub Gorefist".into());
        c.desc = Some("general of the Vile Host".into());
        c.general = Some(u32::MAX);
        g.creature_died(c);
        let skull = g.items.values().find(|it| it.kind == ItemKind::Trophy).expect("no trophy").clone();
        assert_eq!(skull.name(), "skull of Snodub Gorefist, general of the Vile Host");
        assert!(g.trophy_vengeance("Vile Host").contains("skull of Snodub Gorefist"));
        assert!(g.trophy_vengeance("Someone Else").is_empty());
        // Set up on a display.
        let site = (2..20)
            .flat_map(|r| (-r..=r).flat_map(move |k| [e.offset(k, -r, 0), e.offset(k, r, 0), e.offset(-r, k, 0), e.offset(r, k, 0)]))
            .find(|&p| g.can_place(BuildingKind::Display, p) && crate::path::astar(&g.world, e, p, 20_000, crate::world::Mover::Dwarf).is_some())
            .expect("nowhere to put it");
        assert!(g.place_building(BuildingKind::Display, site));
        let bld = g.world.t(site).building;
        for _ in 0..DAY * 2 {
            g.step();
            if g.buildings[&bld].built {
                break;
            }
        }
        let b = &g.buildings[&bld];
        assert!(b.built, "the display was never set up");
        assert_eq!(b.name.as_deref(), Some(skull.label.as_deref().unwrap()));
        assert!(g.craft_lines(bld).iter().any(|l| l.contains("skull of Snodub")));
        assert!(g.trophy_vengeance("Vile Host").contains("skull of Snodub Gorefist"), "a trophy on display still calls them");
        // Those who pass it admire it.
        g.dwarves[1].pos = site.offset(1, 0, 0);
        g.trophies_daily();
        assert!(g.dwarves[1].thoughts.iter().any(|t| t.text.starts_with("admired the skull of Snodub")));
    }

    #[test]
    fn the_jeweler_sets_a_gem() {
        let mut g = Game::new(77);
        g.creatures.clear();
        let e = g.embark;
        let shop = (2..25)
            .flat_map(|r| (-r..=r).flat_map(move |k| [e.offset(k, -r, 0), e.offset(k, r, 0), e.offset(-r, k, 0), e.offset(r, k, 0)]))
            .find(|&p| g.can_place(BuildingKind::Craftsdwarf, p) && g.place_building(BuildingKind::Craftsdwarf, p))
            .map(|p| g.world.t(p).building)
            .expect("no room for a workshop");
        g.buildings.get_mut(&shop).unwrap().built = true;
        g.spawn_item(ItemKind::Gem, Mat::Amethyst, e);
        let table = g.spawn_item(ItemKind::Table, Mat::Granite, e);
        let maker = g.dwarves[3].id;
        {
            let t = g.items.get_mut(&table).unwrap();
            t.quality = 3;
            t.maker = Some(maker);
        }
        let before = g.items[&table].value();
        for d in g.dwarves.iter_mut() {
            d.labors[crate::dwarf::Labor::Crafting.idx()] = true;
        }
        g.add_order(shop, Recipe::Encrust);
        let mut set = None;
        for _ in 0..DAY * 2 {
            g.step();
            set = g.items.values().find(|it| it.kind == ItemKind::Table && it.decor.is_some()).cloned();
            if set.is_some() {
                break;
            }
        }
        let set = set.expect("no gem was ever set");
        assert_eq!(set.decor, Some(Mat::Amethyst));
        assert_eq!((set.quality, set.maker), (3, Some(maker)), "the piece keeps its quality and maker");
        assert_eq!(set.value(), before * 3 / 2);
        assert!(set.name().contains("amethyst-studded"), "{}", set.name());
        assert!(!g.items.values().any(|it| it.kind == ItemKind::Gem), "the gem went into it");
        let lines = g.craft_lines(set.id);
        assert!(lines.iter().any(|l| l.contains("set with amethyst")), "{:?}", lines);
        assert!(lines.iter().any(|l| l.contains("mark of its maker")), "{:?}", lines);
    }
}
