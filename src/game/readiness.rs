//! Arms and kit: what a bolt of each metal does, whether a soldier is ready
//! for a fight, and the overseer seeing that they are before the first siege.

use super::ambitions::{Ambition, Sense};
use super::military::UniformWeapon;
use super::overseer::Overseer;
use super::troubles::Lack;
use super::*;
use crate::items::Slot;
use serde::{Deserialize, Serialize};

/// One piece of a soldier's kit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot3 {
    /// Worn or held.
    Worn,
    /// Not worn, but a free fitting item (or a better weapon) is in the stores.
    InStock,
    None,
}

/// What a soldier has of the four pieces their uniform asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kit {
    pub weapon: Slot3,
    pub helm: Slot3,
    pub mail: Slot3,
    pub shield: Slot3,
}

/// A piece of kit, for a shortfall (`Lack::Kit`) and the squads screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KitPart {
    Weapon,
    Helm,
    Mail,
    Shield,
}

impl KitPart {
    pub const ALL: [KitPart; 4] = [KitPart::Weapon, KitPart::Helm, KitPart::Mail, KitPart::Shield];

    pub fn plural(self) -> &'static str {
        match self {
            KitPart::Weapon => "weapons",
            KitPart::Helm => "helms",
            KitPart::Mail => "mail",
            KitPart::Shield => "shields",
        }
    }

    /// The squads screen's column letter.
    pub fn letter(self) -> char {
        match self {
            KitPart::Weapon => 'W',
            KitPart::Helm => 'H',
            KitPart::Mail => 'M',
            KitPart::Shield => 'S',
        }
    }
}

impl Kit {
    pub fn get(&self, p: KitPart) -> Slot3 {
        match p {
            KitPart::Weapon => self.weapon,
            KitPart::Helm => self.helm,
            KitPart::Mail => self.mail,
            KitPart::Shield => self.shield,
        }
    }
}

/// How much a uniform wants a weapon (0: not at all), as `equip_soldier` scores it.
fn weapon_score(uniform: UniformWeapon, it: &Item) -> f32 {
    if !uniform.accepts(it.kind) {
        return 0.0;
    }
    it.kind.weapon_damage().map_or(0.0, |w| w * it.mat.weapon_mult()) + 0.1
}

fn metal(m: Mat) -> bool {
    matches!(m, Mat::Copper | Mat::Bronze | Mat::Iron | Mat::Steel | Mat::Gold)
}

impl Game {
    /// What one bolt does before skill and armour: wooden bolts (or none
    /// known) 9, bone 9.5, copper 10, bronze 11, iron 12, steel 14.
    pub(super) fn bolt_damage(mat: Option<Mat>) -> f32 {
        match mat {
            Some(Mat::Steel) => 14.0,
            Some(Mat::Iron) => 12.0,
            Some(Mat::Bronze) => 11.0,
            Some(Mat::Copper) => 10.0,
            Some(Mat::Bone) | Some(Mat::Gold) => 9.5,
            _ => 9.0,
        }
    }

    /// A work order for `metal` forged crossbows or bolt bundles. Stock is
    /// counted by kind whatever it's made of, so the wooden ones on hand are
    /// added to the mark: otherwise a stack of wooden bolts would keep the
    /// forge idle.
    pub(super) fn metal_arms_order(&self, recipe: Recipe, metal: u32) -> (Recipe, u32) {
        let kind = recipe.product().0;
        let held: HashSet<u32> = self.dwarves.iter().filter(|d| d.alive).filter_map(|d| d.held).collect();
        let wooden = self.items.values().filter(|it| it.kind == kind && it.mat == Mat::Wood && !held.contains(&it.id)).count() as u32;
        (recipe, metal + wooden)
    }

    /// A soldier's kit: each piece worn, waiting in the stores, or missing.
    /// (`InStock` is what `equip_soldier` would go and fetch.)
    pub fn kit(&self, dwarf: u32) -> Kit {
        let none = Kit { weapon: Slot3::None, helm: Slot3::None, mail: Slot3::None, shield: Slot3::None };
        let Some(d) = self.dwarves.iter().find(|d| d.id == dwarf && d.alive) else { return none };
        let (uniform, armor) = self.uniform_of(dwarf);
        let current = d.held.and_then(|id| self.items.get(&id)).map_or(0.0, |it| weapon_score(uniform, it));
        let worn: Vec<Slot> = d.armor.iter().filter_map(|id| self.items.get(id).and_then(|it| it.kind.armor_slot())).collect();
        let mut best = 0.0f32;
        let mut stocked: Vec<Slot> = Vec::new();
        for it in self.items.values().filter(|it| it.free()) {
            best = best.max(weapon_score(uniform, it));
            if let Some(s) = it.kind.armor_slot() {
                if !stocked.contains(&s) {
                    stocked.push(s);
                }
            }
        }
        let weapon = if current > 0.0 {
            if best > current + 0.5 { Slot3::InStock } else { Slot3::Worn }
        } else if best > 0.0 {
            Slot3::InStock
        } else {
            Slot3::None
        };
        let piece = |s: Slot| {
            if worn.contains(&s) {
                Slot3::Worn
            } else if armor && stocked.contains(&s) {
                Slot3::InStock
            } else {
                Slot3::None
            }
        };
        Kit { weapon, helm: piece(Slot::Head), mail: piece(Slot::Body), shield: piece(Slot::Shield) }
    }

    /// Fit to fight: a metal weapon in hand and at least one piece of armour on.
    pub fn kit_ready(&self, dwarf: u32) -> bool {
        let Some(d) = self.dwarves.iter().find(|d| d.id == dwarf && d.alive) else { return false };
        let armed = d.held.and_then(|id| self.items.get(&id)).map_or(false, |it| it.kind.weapon_damage().is_some() && it.kind != ItemKind::Pick && metal(it.mat));
        let armoured = d.armor.iter().any(|id| self.items.get(id).map_or(false, |it| it.kind.armor_slot().is_some()));
        armed && armoured
    }

    /// Every soldier with kit waiting in the stores drops what they're doing
    /// and goes to put it on. Only idling, drill, a station or ordinary work
    /// gives way: never a need, a fight, a rescue or treatment, a lever, a
    /// mood or a breakdown, nor a soldier in jail. How many went.
    pub(super) fn kit_up(&mut self) -> usize {
        let mut sent = 0;
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            let busy = d.mood.is_some() || d.insane || d.tantrum > 0 || d.jailed_until > 0 || d.body.unconscious();
            if !d.alive || !d.soldier || busy || !Self::kit_interruptible(&d.task) {
                continue;
            }
            let k = self.kit(d.id);
            if !KitPart::ALL.iter().any(|&p| k.get(p) == Slot3::InStock) {
                continue;
            }
            // Only let go of the old task once there is kit they can reach.
            let mut d = std::mem::take(&mut self.dwarves[i]);
            let (old_task, old_path) = (d.task.clone(), d.path.clone());
            if self.equip_soldier(&mut d) {
                let fetch = std::mem::replace(&mut d.task, old_task);
                let path = std::mem::replace(&mut d.path, old_path);
                self.release_task(&mut d);
                d.task = fetch;
                d.path = path;
                sent += 1;
            }
            self.dwarves[i] = d;
        }
        sent
    }

    /// The tasks `kit_up` may cut short.
    fn kit_interruptible(t: &Task) -> bool {
        matches!(
            t,
            Task::Idle
                | Task::Wander { .. }
                | Task::Station { .. }
                | Task::Train
                | Task::Dig { .. }
                | Task::Haul { .. }
                | Task::Construct { .. }
                | Task::Craft { .. }
                | Task::Plant { .. }
                | Task::Harvest { .. }
                | Task::Gather { .. }
                | Task::Clean { .. }
                | Task::Pond { .. }
                | Task::Fish { .. }
                | Task::BuildTile { .. }
        )
    }

    /// Now and then while drilling or holding a station (which otherwise go
    /// on for a long while): fetch any better kit. True if they went for it.
    pub(super) fn kit_check(&mut self, d: &mut Dwarf) -> bool {
        if self.tick % 300 != d.id as u64 % 300 || !d.soldier {
            return false;
        }
        if self.equip_soldier(d) {
            d.working = false;
            return true;
        }
        false
    }

    /// The overseer's daily look at the squads (overseer_hooks.rs): kit in
    /// the stores is put on, and kit nobody has is a shortfall to see to.
    pub(super) fn ov_readiness(&mut self, o: &mut Overseer) {
        let sent = self.kit_up();
        if sent > 0 {
            let text = o.voiced(&format!("Had {} soldier{} fetch their kit from the stores.", sent, if sent == 1 { "" } else { "s" }));
            Self::note_ov(o, self.tick, text);
        }
        for p in KitPart::ALL {
            if self.kit_short(p) > 0 {
                // (Counted once per shortfall, not every day it lasts.)
                if !self.troubles.iter().any(|t| t.0 == Lack::Kit(p)) {
                    self.count("kit_shortfalls", 1);
                }
                self.trouble(Lack::Kit(p));
            }
        }
    }

    /// Soldiers with nothing in the stores for this piece (armour only when
    /// their uniform asks for it).
    pub(super) fn kit_short(&self, p: KitPart) -> u32 {
        let ids: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && d.soldier).map(|d| d.id).collect();
        ids.into_iter().filter(|&id| self.kit(id).get(p) == Slot3::None && (p == KitPart::Weapon || self.uniform_of(id).1)).count() as u32
    }

    /// Most of the soldiers with no weapon to fetch are marksdwarves.
    fn short_marksdwarves(&self) -> bool {
        let ids: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && d.soldier).map(|d| d.id).collect();
        let short: Vec<u32> = ids.into_iter().filter(|&id| self.kit(id).weapon == Slot3::None).collect();
        let bows = short.iter().filter(|&&id| self.uniform_of(id).0 == UniformWeapon::Crossbow).count();
        bows > 0 && bows * 2 >= short.len()
    }

    /// What to order for a kit shortfall, and how many: from the forge when
    /// there is one, else leather (or wooden crossbows) from the workshops.
    /// Nothing for shields without a forge.
    pub(super) fn kit_order(&self, p: KitPart) -> Option<(Recipe, u32)> {
        let forge = self.buildings.values().any(|b| b.built && matches!(b.kind, BuildingKind::Forge | BuildingKind::MagmaForge));
        let recipe = match (p, forge) {
            (KitPart::Weapon, true) if self.short_marksdwarves() => Recipe::ForgeCrossbow,
            (KitPart::Weapon, true) => Recipe::ForgeAxe,
            (KitPart::Weapon, false) => Recipe::WoodCrossbow,
            (KitPart::Helm, true) => Recipe::ForgeHelm,
            (KitPart::Helm, false) => Recipe::LeatherHelm,
            (KitPart::Mail, true) => Recipe::ForgeBreastplate,
            (KitPart::Mail, false) => Recipe::LeatherArmor,
            (KitPart::Shield, true) => Recipe::ForgeShield,
            (KitPart::Shield, false) => return None,
        };
        Some((recipe, self.kit_short(p).max(1)))
    }

    /// The `Unarmed` sense: the share of soldiers not fit to fight. It pulls
    /// towards Muster and holds the squad back from expeditions.
    pub(super) fn unarmed_sense(&self, o: &Overseer) -> Option<(Sense, f64, Vec<(Ambition, f64)>)> {
        let _ = o;
        let soldiers: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && d.soldier).map(|d| d.id).collect();
        if soldiers.is_empty() {
            return None;
        }
        let share = soldiers.iter().filter(|&&id| !self.kit_ready(id)).count() as f64 / soldiers.len() as f64;
        Some((Sense::Unarmed, share, vec![(Ambition::Muster, self.knob("unarmed.muster", 1.2)), (Ambition::Venture, -self.knob("unarmed.venture", 4.0))]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A squad of three holds a station with nothing to wear; iron axes,
    /// helms and breastplates then turn up in a bin in a stockpile. When the
    /// scouts' warning comes, all three are in them well before the army.
    #[test]
    fn kit_is_on_before_the_first_siege() {
        let mut g = Game::new(21);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        let ids: Vec<u32> = g.dwarves.iter().filter(|d| !d.is_child()).take(3).map(|d| d.id).collect();
        // (Nothing to pick up yet: no weapons or armour anywhere.)
        g.items.retain(|_, it| !it.kind.is_equipment() || it.carried_by.is_some());
        let s = g.new_squad();
        for &id in &ids {
            g.set_squad_member(s, id, true);
        }
        g.order_station(s, e);
        for _ in 0..DAY / 2 {
            g.step();
        }
        for &id in &ids {
            assert!(!g.kit_ready(id));
        }
        // The kit arrives, boxed.
        let a = g.flat_patch(3, 3, 6);
        assert!(g.add_zone(a, a.offset(2, 2, 0), ZoneKind::Stockpile(StockFilter::Equipment)) > 0);
        let bin = g.spawn_item(ItemKind::Bin, Mat::Wood, a);
        let at = g.items[&bin].pos;
        for kind in [ItemKind::Axe, ItemKind::Helm, ItemKind::Breastplate] {
            for _ in 0..3 {
                let it = g.spawn_item(kind, Mat::Iron, at);
                g.items.get_mut(&it).unwrap().container = Some(bin);
            }
        }
        let k = g.kit(ids[0]);
        assert_eq!((k.helm, k.mail, k.shield), (Slot3::InStock, Slot3::InStock, Slot3::None));
        // The scouts' warning.
        g.tick = g.tick.max(SEASON * 2 + 1);
        for _ in 0..100 {
            g.maybe_siege();
            if g.incoming.is_some() {
                break;
            }
        }
        let arrive = g.incoming.as_ref().expect("no warning came").0;
        // At once, they all go for it.
        for &id in &ids {
            let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
            assert!(matches!(d.task, Task::Equip { .. }), "{} is still {:?}", d.name, d.task);
        }
        for _ in 0..300 {
            g.step();
        }
        assert!(g.tick < arrive, "the army came first");
        for &id in &ids {
            let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
            let held = d.held.and_then(|i| g.items.get(&i)).map(|it| (it.kind, it.mat));
            let k = g.kit(id);
            assert_eq!(held, Some((ItemKind::Axe, Mat::Iron)), "{} holds {:?} ({:?})", d.name, held, d.task);
            assert_eq!((k.weapon, k.helm, k.mail), (Slot3::Worn, Slot3::Worn, Slot3::Worn), "{}: {:?} ({:?})", d.name, k, d.task);
            assert!(g.kit_ready(id));
        }
        assert!(g.items.values().filter(|it| it.carried_by.is_some()).all(|it| it.container.is_none()), "worn kit is out of its bin");
    }

    /// Kit nobody has is a trouble the overseer sees to: no forge, so leather.
    #[test]
    fn a_kit_shortfall_is_a_trouble() {
        let mut g = Game::new(5);
        g.set_overseer(Some(super::super::overseer::Style::Steady));
        g.items.retain(|_, it| it.kind.armor_slot().is_none());
        let id = g.dwarves[0].id;
        let s = g.new_squad();
        g.set_squad_member(s, id, true);
        let mut o = g.overseer.take().unwrap();
        g.ov_readiness(&mut o);
        assert!(g.troubles.iter().any(|t| t.0 == Lack::Kit(KitPart::Helm)));
        assert!(g.annals.counters.get("kit_shortfalls").copied().unwrap_or(0) >= 2);
        assert_eq!(g.kit_order(KitPart::Mail), Some((Recipe::LeatherArmor, 1)));
        assert_eq!(g.kit_order(KitPart::Shield), None);
        // The unarmed soldier weighs on the brain.
        let (sense, share, pulls) = g.unarmed_sense(&o).unwrap();
        assert_eq!((sense, share), (Sense::Unarmed, 1.0));
        assert!(pulls.iter().any(|p| p.0 == Ambition::Muster && p.1 > 0.0));
        g.overseer = Some(o);
    }

    /// Kit in the stores never pulls a soldier off a rescue, a mood, a
    /// lever or out of jail; a hauler at ordinary work does go.
    #[test]
    fn kit_up_spares_what_matters() {
        let mut g = Game::new(21);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        let ids: Vec<u32> = g.dwarves.iter().filter(|d| !d.is_child()).take(4).map(|d| d.id).collect();
        g.items.retain(|_, it| !it.kind.is_equipment() || it.carried_by.is_some());
        let s = g.new_squad();
        for &id in &ids {
            g.set_squad_member(s, id, true);
        }
        let at = g.flat_patch(2, 2, 6);
        for _ in 0..4 {
            g.spawn_item(ItemKind::Helm, Mat::Iron, at);
        }
        let patient = g.dwarves.iter().find(|d| !ids.contains(&d.id)).unwrap().id;
        let set = |g: &mut Game, id: u32, t: Task| g.dwarves.iter_mut().find(|d| d.id == id).unwrap().task = t;
        set(&mut g, ids[0], Task::Rescue { patient, bed: None });
        set(&mut g, ids[1], Task::PullLever { bld: 0 });
        set(&mut g, ids[2], Task::Wander { target: e });
        g.dwarves.iter_mut().find(|d| d.id == ids[2]).unwrap().jailed_until = g.tick + DAY;
        set(&mut g, ids[3], Task::Wander { target: e });
        assert_eq!(g.kit(ids[3]).helm, Slot3::InStock);
        assert_eq!(g.kit_up(), 1);
        let task = |g: &Game, id: u32| g.dwarves.iter().find(|d| d.id == id).unwrap().task.clone();
        assert!(matches!(task(&g, ids[0]), Task::Rescue { .. }), "the rescuer put the patient down");
        assert!(matches!(task(&g, ids[1]), Task::PullLever { .. }));
        assert!(matches!(task(&g, ids[2]), Task::Wander { .. }), "walked out of jail");
        assert!(matches!(task(&g, ids[3]), Task::Equip { .. }));
    }

    /// No kit within reach: the soldier keeps doing what they were doing.
    #[test]
    fn kit_up_keeps_the_task_when_nothing_can_be_fetched() {
        let mut g = Game::new(21);
        g.items.retain(|_, it| !it.kind.is_equipment() || it.carried_by.is_some());
        let id = g.dwarves.iter().find(|d| !d.is_child()).unwrap().id;
        let s = g.new_squad();
        g.set_squad_member(s, id, true);
        // A helm that's free but nowhere anyone can walk to.
        let it = g.spawn_item(ItemKind::Helm, Mat::Iron, g.embark);
        let buried = (3..20)
            .map(|k| g.embark.offset(0, 0, -k))
            .find(|&p| g.world.in_bounds(p) && [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|&(x, y)| g.world.solid(p.offset(x, y, 0)) && g.world.solid(p.offset(x, y, 1))))
            .expect("no solid rock below the embark");
        g.items.get_mut(&it).unwrap().pos = buried;
        let target = g.embark;
        g.dwarves.iter_mut().find(|d| d.id == id).unwrap().task = Task::Wander { target };
        assert_eq!(g.kit(id).helm, Slot3::InStock);
        assert_eq!(g.kit_up(), 0);
        assert!(matches!(g.dwarves.iter().find(|d| d.id == id).unwrap().task, Task::Wander { .. }));
    }

    /// A stationed squad with no warning still puts on kit that turns up:
    /// the check while holding a station.
    #[test]
    fn a_stationed_squad_picks_up_new_kit() {
        let mut g = Game::new(21);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        let ids: Vec<u32> = g.dwarves.iter().filter(|d| !d.is_child()).take(2).map(|d| d.id).collect();
        g.items.retain(|_, it| !it.kind.is_equipment() || it.carried_by.is_some());
        let s = g.new_squad();
        for &id in &ids {
            g.set_squad_member(s, id, true);
        }
        g.order_station(s, e);
        for _ in 0..DAY / 4 {
            g.step();
        }
        let at = g.flat_patch(2, 2, 6);
        for _ in 0..2 {
            g.spawn_item(ItemKind::Helm, Mat::Iron, at);
        }
        for _ in 0..300 + 200 {
            g.step();
        }
        assert!(g.incoming.is_none());
        for &id in &ids {
            assert_eq!(g.kit(id).helm, Slot3::Worn, "{:?}", g.dwarves.iter().find(|d| d.id == id).unwrap().task);
        }
    }

    /// A crossbow squad short of weapons gets crossbows from the forge, not axes.
    #[test]
    fn a_crossbow_shortfall_orders_crossbows() {
        let mut g = Game::new(5);
        g.items.retain(|_, it| it.kind.weapon_damage().is_none() || it.carried_by.is_some());
        let id = g.dwarves.iter().find(|d| !d.is_child()).unwrap().id;
        let s = g.new_squad();
        g.set_squad_member(s, id, true);
        g.squads[s].weapon = UniformWeapon::Crossbow;
        assert_eq!(g.kit_order(KitPart::Weapon).map(|o| o.0), Some(Recipe::WoodCrossbow));
        let e = g.embark;
        let spot = (2..25)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).flat_map(move |dx| (-1..=1).map(move |dz| e.offset(dx, dy, dz)))))
            .find(|&p| g.can_place(BuildingKind::Forge, p))
            .expect("no room for a forge");
        assert!(g.place_building(BuildingKind::Forge, spot));
        let f = g.world.t(spot).building;
        g.buildings.get_mut(&f).unwrap().built = true;
        assert_eq!(g.kit_order(KitPart::Weapon).map(|o| o.0), Some(Recipe::ForgeCrossbow));
        g.squads[s].weapon = UniformWeapon::Axe;
        assert_eq!(g.kit_order(KitPart::Weapon).map(|o| o.0), Some(Recipe::ForgeAxe));
    }
}
