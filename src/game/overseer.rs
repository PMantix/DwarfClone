//! The overseer: an AI that plays the fortress through the same commands a
//! player has (designations, buildings, zones, work orders, squads, posts,
//! trade, expeditions), in one of several styles. Turn it on with `A` in a
//! fortress, watch one from the title screen, or run many from the command
//! line to see which styles end in ruin and which in glory.

use super::*;
use crate::buildings::Recipe;
use super::ambitions::Ambition;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Style {
    /// A bit of everything, sensibly.
    Steady,
    /// Fine halls, engravings, taverns and temples, and in time a monument.
    Builder,
    /// Digs deep for ore, gems, caverns and the magma sea.
    Delver,
    /// A big army early, and expeditions to raid and hunt.
    Warlord,
    /// Wide halls, no soldiers, no coffins, no burrows. Fast, and doomed?
    Reckless,
}

impl Style {
    pub const ALL: [Style; 5] = [Style::Steady, Style::Builder, Style::Delver, Style::Warlord, Style::Reckless];

    pub fn name(self) -> &'static str {
        match self {
            Style::Steady => "Steady",
            Style::Builder => "Builder",
            Style::Delver => "Delver",
            Style::Warlord => "Warlord",
            Style::Reckless => "Reckless",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Style::Steady => "a bit of everything, sensibly",
            Style::Builder => "fine halls, engravings, taverns, a monument",
            Style::Delver => "digs deep for ore, caverns and magma",
            Style::Warlord => "a big army early, raids and hunts",
            Style::Reckless => "wide halls, no soldiers, no coffins",
        }
    }

    pub fn parse(s: &str) -> Option<Style> {
        Style::ALL.iter().copied().find(|st| st.name().eq_ignore_ascii_case(s))
    }

    /// Share of adults kept under arms, and the population to start at.
    fn army(self) -> (f64, usize) {
        match self {
            Style::Warlord => (0.3, 7),
            Style::Steady => (0.2, 9),
            Style::Delver => (0.15, 10),
            Style::Builder => (0.12, 10),
            Style::Reckless => (0.0, usize::MAX),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomUse {
    Workshops,
    Stockpile,
    Dining,
    Dormitory,
    Meeting,
    Tavern,
    Temple,
    Hospital,
    Tomb,
    Barracks,
    Office,
    Jail,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub kind: RoomUse,
    pub a: Pos,
    pub b: Pos,
    /// 0 dug out (designated), 1 fitted out.
    pub stage: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Overseer {
    pub style: Style,
    /// The stair at the surface, and its foot on the main level.
    pub top: Option<Pos>,
    pub base: Option<Pos>,
    pub rooms: Vec<Room>,
    /// How far the main corridor has been dug, in room slots.
    pub slots: i32,
    /// One-off projects already done.
    pub done: Vec<String>,
    pub peak_pop: usize,
    pub peak_wealth: u32,
    /// When the drawbridge was last raised (0 when it's down).
    #[serde(default)]
    pub raised_at: u64,
    /// What it has its heart set on now, since when, and a mark to measure
    /// progress against (wealth at the start, shafts sunk...).
    #[serde(default)]
    pub ambition: Option<Ambition>,
    #[serde(default)]
    pub ambition_since: u64,
    #[serde(default)]
    pub ambition_mark: u32,
    /// Every ambition it has had, in order.
    #[serde(default)]
    pub ambitions: Vec<(u64, Ambition)>,
    /// Rooms an ambition has asked for, dug before the rest of the plan.
    #[serde(default)]
    pub wants: Vec<RoomUse>,
    /// How far through the style's room plan it has got.
    #[serde(default)]
    pub planned: usize,
    /// When an army last came.
    #[serde(default)]
    pub last_siege: u64,
    /// What it decided, newest last.
    pub notes: Vec<(u64, String)>,
}

impl Overseer {
    pub fn new(style: Style) -> Overseer {
        Overseer {
            style,
            top: None,
            base: None,
            rooms: Vec::new(),
            slots: 0,
            done: Vec::new(),
            peak_pop: 0,
            peak_wealth: 0,
            raised_at: 0,
            ambition: None,
            ambition_since: 0,
            ambition_mark: 0,
            ambitions: Vec::new(),
            wants: Vec::new(),
            planned: 0,
            last_siege: 0,
            notes: Vec::new(),
        }
    }

    pub(super) fn did(&self, what: &str) -> bool {
        self.done.iter().any(|d| d == what)
    }
}

/// The order rooms are dug in, by style.
fn plan(style: Style) -> Vec<RoomUse> {
    use RoomUse::*;
    match style {
        Style::Builder => vec![Workshops, Stockpile, Dining, Dormitory, Meeting, Workshops, Tavern, Temple, Hospital, Tomb, Office, Stockpile, Dormitory, Barracks, Jail],
        Style::Warlord => vec![Workshops, Stockpile, Barracks, Dining, Dormitory, Workshops, Hospital, Meeting, Tomb, Office, Stockpile, Dormitory, Tavern, Jail, Temple],
        Style::Delver => vec![Workshops, Stockpile, Dining, Dormitory, Workshops, Stockpile, Hospital, Meeting, Tomb, Barracks, Office, Dormitory, Tavern, Temple, Jail],
        Style::Reckless => vec![Workshops, Stockpile, Dining, Dormitory, Workshops, Tavern, Meeting, Stockpile, Dormitory, Temple],
        Style::Steady => vec![Workshops, Stockpile, Dining, Dormitory, Meeting, Workshops, Hospital, Tomb, Barracks, Tavern, Office, Stockpile, Dormitory, Temple, Jail],
    }
}

const WORKSHOPS: [BuildingKind; 12] = [
    BuildingKind::Carpenter,
    BuildingKind::Mason,
    BuildingKind::Still,
    BuildingKind::Kitchen,
    BuildingKind::Mechanic,
    BuildingKind::Craftsdwarf,
    BuildingKind::Butcher,
    BuildingKind::WoodFurnace,
    BuildingKind::Smelter,
    BuildingKind::Forge,
    BuildingKind::Loom,
    BuildingKind::Tannery,
];

impl Game {
    /// Hand the fortress to the overseer (or take it back with `None`).
    pub fn set_overseer(&mut self, style: Option<Style>) {
        match style {
            Some(s) => {
                let mut o = self.overseer.take().unwrap_or_else(|| Overseer::new(s));
                o.style = s;
                self.overseer = Some(o);
                self.msg(&format!("The overseer takes charge ({}: {}).", s.name(), s.blurb()), (150, 220, 255));
            }
            None => {
                if self.overseer.take().is_some() {
                    self.msg("You take back the fortress from the overseer.", (150, 220, 255));
                }
            }
        }
    }

    pub(super) fn note_ov(o: &mut Overseer, tick: u64, text: String) {
        Self::note(o, tick, text);
    }

    fn note(o: &mut Overseer, tick: u64, text: String) {
        o.notes.push((tick, text));
        if o.notes.len() > 60 {
            o.notes.remove(0);
        }
    }

    /// Every little while: look over the fortress and give orders.
    pub(super) fn oversee(&mut self) {
        let Some(mut o) = self.overseer.take() else { return };
        o.peak_pop = o.peak_pop.max(self.alive_count());
        if self.tick % DAY < 150 {
            o.peak_wealth = o.peak_wealth.max(self.wealth());
        }
        if o.base.is_none() {
            self.ov_setup(&mut o);
        } else {
            self.ov_rooms(&mut o);
            self.ov_orders(&o);
            self.ov_posts();
            self.ov_army(&mut o);
            if self.tick % DAY < 150 {
                self.ov_daily(&mut o);
            }
        }
        self.overseer = Some(o);
    }

    /// Often: watch for enemies, and shut the gate in time.
    pub(super) fn oversee_alert(&mut self) {
        let Some(mut o) = self.overseer.take() else { return };
        if o.base.is_some() {
            self.ov_alert(&mut o);
        }
        self.overseer = Some(o);
    }

    /// A spot near `c` where `kind` can be placed.
    pub(super) fn ov_spot(&self, kind: BuildingKind, c: Pos, r: i32) -> Option<Pos> {
        for d in 0..=r {
            for dy in -d..=d {
                for dx in -d..=d {
                    if dx.abs() != d && dy.abs() != d {
                        continue;
                    }
                    for dz in [0, -1, 1] {
                        let p = c.offset(dx, dy, dz);
                        if self.world.in_bounds(p) && self.can_place(kind, p) {
                            return Some(p);
                        }
                    }
                }
            }
        }
        None
    }

    /// Day one: chop trees, stockpile and farm by the wagon, a few workshops
    /// on the surface, and a stair down into the rock.
    fn ov_setup(&mut self, o: &mut Overseer) {
        let e = self.embark;
        let tick = self.tick;
        self.designate(e.offset(-15, -15, -1), e.offset(15, 15, 1), Desig::Chop);
        // Wood and stone by the wagon; food and drink go below, behind the gate.
        self.add_zone(e.offset(-8, 3, -1), e.offset(-5, 9, 1), ZoneKind::Stockpile(StockFilter::Wood));
        self.add_zone(e.offset(-4, 3, -1), e.offset(-1, 9, 1), ZoneKind::Stockpile(StockFilter::Stone));
        self.add_zone(e.offset(-8, -8, -1), e.offset(-2, -3, 1), ZoneKind::Farm);
        self.add_zone(e.offset(2, -9, -1), e.offset(8, -4, 1), ZoneKind::Farm);
        // Wood, stone, drink and food first, right by the wagon.
        for kind in [BuildingKind::Carpenter, BuildingKind::Still, BuildingKind::Kitchen, BuildingKind::Mason, BuildingKind::Mechanic] {
            if let Some(p) = self.ov_spot(kind, e.offset(4, 4, 0), 14) {
                self.place_building(kind, p);
            }
        }
        // The stair: on open ground beside the wagon, down into the rock.
        if let Some(m) = self.candidates(Position::Manager).first().copied() {
            self.appoint(Position::Manager, Some(m));
        }
        let s = (1..12).flat_map(|i| [e.offset(i, 0, 0), e.offset(0, i, 0), e.offset(-i, 0, 0)]).find(|&p| {
            let Some(t) = self.world.get(p) else { return false };
            t.kind == TileKind::Floor && t.building == 0 && t.zone == 0
        });
        let Some(s) = s else {
            self.overseer = None;
            return;
        };
        let depth = if o.style == Style::Delver { 4 } else { 3 };
        let base = s.offset(0, 0, -depth);
        if base.z < 2 {
            return;
        }
        self.designate(s, base, Desig::StairUD);
        o.top = Some(s);
        o.base = Some(base);
        o.slots = 0;
        if let Some(m) = self.candidates(Position::Manager).first().copied() {
            self.appoint(Position::Manager, Some(m));
        }
        Self::note(o, tick, format!("Struck the earth: a stair down {} levels, farms and a stockpile by the wagon.", depth));
    }

    fn ov_dims(o: &Overseer) -> (i32, i32) {
        if o.style == Style::Reckless {
            (11, 9)
        } else {
            (7, 6)
        }
    }

    /// Rooms each level's corridor has room for, on each side.
    pub(super) fn ov_per_level(&self, o: &Overseer) -> i32 {
        let b = o.base.unwrap();
        let (w, _) = Self::ov_dims(o);
        ((self.world.w - 3 - (b.x + 2)) / (w + 1)).clamp(1, 6)
    }

    /// The box for room `i`, and its level. Each level is a corridor of
    /// rooms on both sides; deeper levels are reached by a stair inside
    /// the gate (the hub).
    pub(super) fn ov_room_box(&self, o: &Overseer, i: usize) -> Option<(Pos, Pos, i32)> {
        let b = o.base.unwrap();
        let (w, d) = Self::ov_dims(o);
        let per = self.ov_per_level(o) as usize;
        let level = (i / (2 * per)) as i32;
        let within = i % (2 * per);
        let (slot, north) = ((within / 2) as i32, within % 2 == 0);
        let z = b.z - 2 * level;
        if z < 3 {
            return None;
        }
        let x0 = b.x + 2 + (w + 1) * slot;
        Some(if north {
            (Pos::new(x0, b.y - d, z), Pos::new(x0 + w - 1, b.y - 1, z), level)
        } else {
            (Pos::new(x0, b.y + 2, z), Pos::new(x0 + w - 1, b.y + 1 + d, z), level)
        })
    }

    /// The stair inside the gate that leads to the deeper levels.
    pub(super) fn ov_hub(o: &Overseer) -> Pos {
        o.base.unwrap().offset(9, 1, 0)
    }

    /// Open up a level: its corridor, and the stair down to it.
    fn ov_open_level(&mut self, o: &mut Overseer, level: i32) {
        let b = o.base.unwrap();
        let (w, _) = Self::ov_dims(o);
        let per = self.ov_per_level(o);
        let z = b.z - 2 * level;
        let x_end = b.x + 2 + (w + 1) * per;
        if level == 0 {
            self.designate(b.offset(1, 0, 0), Pos::new(x_end, b.y + 1, z), Desig::Dig);
        } else {
            let hub = Self::ov_hub(o);
            self.designate(Pos::new(b.x + 2, b.y, z), Pos::new(x_end, b.y + 1, z), Desig::Dig);
            self.designate(hub.offset(0, 0, -2 * (level - 1)), Pos::new(hub.x, hub.y, z), Desig::StairUD);
            // The shelter reaches down too.
            if let Some(i) = self.burrows.iter().position(|bw| Some(bw.bit) == self.alert_burrow) {
                self.paint_burrow(i, Pos::new(b.x, b.y - 10, z), Pos::new(x_end + 1, b.y + 10, z + 1), true);
            }
            Self::note(o, self.tick, format!("Opened a new level of halls, {} below the first.", 2 * level));
        }
        o.slots = o.slots.max(per);
        o.done.push(format!("level{}", level));
    }

    fn dug_share(&self, a: Pos, b: Pos) -> f64 {
        let mut n = 0;
        let mut dug = 0;
        for p in box_iter(a, b) {
            n += 1;
            if self.world.get(p).map_or(false, |t| !t.kind.solid()) {
                dug += 1;
            }
        }
        dug as f64 / n.max(1) as f64
    }

    /// Dig the next room when the last is under way, and fit out rooms
    /// once they're dug.
    fn ov_rooms(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let plan = plan(o.style);
        let want = 5 + self.alive_count() / 3;
        let digging = o.rooms.iter().filter(|r| r.stage == 0).count();
        let dig_jobs = self.designations.iter().filter(|p| matches!(self.world.t(**p).desig, Desig::Dig | Desig::StairUD)).count();
        // An ambition's rooms first, then the style's plan.
        let next = o.wants.first().copied().map(|k| (k, true)).or_else(|| plan.get(o.planned).map(|&k| (k, false)));
        if let Some((kind, wanted)) = next.filter(|_| (o.rooms.len() < want || !o.wants.is_empty()) && digging < 2 && dig_jobs < 120) {
            if let Some((a, b, level)) = self.ov_room_box(o, o.rooms.len()) {
                if !o.did(&format!("level{}", level)) {
                    self.ov_open_level(o, level);
                }
                self.designate(a, b, Desig::Dig);
                o.rooms.push(Room { kind, a, b, stage: 0 });
                if wanted {
                    o.wants.remove(0);
                } else {
                    o.planned += 1;
                }
                Self::note(o, tick, format!("Dug out a room for a {:?}.", kind).to_lowercase().replace("dug out", "Dug out"));
            } else {
                o.wants.clear();
            }
        }
        for i in 0..o.rooms.len() {
            if o.rooms[i].stage == 0 && self.dug_share(o.rooms[i].a, o.rooms[i].b) >= 0.9 {
                let r = o.rooms[i].clone();
                self.ov_fit(o, &r);
                o.rooms[i].stage = 1;
            }
        }
        // Keep a bed for everyone in the dormitories.
        if self.tick % DAY < 150 {
            let beds = self.buildings.values().filter(|b| b.kind == BuildingKind::Bed).count();
            if beds < self.alive_count() + 1 {
                for r in o.rooms.iter().filter(|r| r.kind == RoomUse::Dormitory && r.stage == 1).cloned().collect::<Vec<_>>() {
                    self.ov_grid(&r, BuildingKind::Bed, 2, 2, (self.alive_count() + 1).saturating_sub(beds));
                }
            }
        }
    }

    /// Place up to `max` of `kind` across a room in a grid.
    fn ov_grid(&mut self, r: &Room, kind: BuildingKind, sx: i32, sy: i32, max: usize) -> usize {
        let mut n = 0;
        let mut y = r.a.y;
        while y <= r.b.y && n < max {
            let mut x = r.a.x;
            while x <= r.b.x && n < max {
                if self.place_building(kind, Pos::new(x, y, r.a.z)) {
                    n += 1;
                }
                x += sx;
            }
            y += sy;
        }
        n
    }

    /// Zone and furnish a freshly dug room.
    fn ov_fit(&mut self, o: &mut Overseer, r: &Room) {
        let tick = self.tick;
        let zone = |k| Some(k);
        let z = match r.kind {
            RoomUse::Workshops => {
                let have: Vec<BuildingKind> = self.buildings.values().map(|b| b.kind).collect();
                let todo: Vec<BuildingKind> = WORKSHOPS.iter().copied().filter(|k| !have.contains(k)).collect();
                let mut placed = 0;
                for (dx, dy) in [(0, 0), (4, 0), (0, 3), (4, 3), (8, 0), (8, 3)] {
                    let Some(&kind) = todo.get(placed) else { break };
                    let p = r.a.offset(dx, dy, 0);
                    if p.x + 2 <= r.b.x && p.y + 2 <= r.b.y && self.place_building(kind, p) {
                        placed += 1;
                    }
                }
                None
            }
            RoomUse::Stockpile => zone(ZoneKind::Stockpile(StockFilter::All)),
            RoomUse::Dining => {
                self.ov_grid(r, BuildingKind::Table, 3, 2, 12);
                let tables: Vec<Pos> = self.buildings.values().filter(|b| b.kind == BuildingKind::Table && box_iter(r.a, r.b).any(|p| p == b.pos)).map(|b| b.pos).collect();
                for t in tables {
                    self.place_building(BuildingKind::Chair, t.offset(1, 0, 0));
                }
                zone(ZoneKind::Dining)
            }
            RoomUse::Dormitory => {
                self.ov_grid(r, BuildingKind::Bed, 2, 2, 12);
                zone(ZoneKind::Dormitory)
            }
            RoomUse::Meeting => zone(ZoneKind::Meeting),
            RoomUse::Tavern => {
                self.ov_grid(r, BuildingKind::Table, 3, 3, 4);
                zone(ZoneKind::Tavern)
            }
            RoomUse::Temple => zone(ZoneKind::Temple),
            RoomUse::Hospital => {
                self.ov_grid(r, BuildingKind::Bed, 3, 3, 3);
                zone(ZoneKind::Hospital)
            }
            RoomUse::Tomb => {
                self.ov_grid(r, BuildingKind::Coffin, 2, 2, 8);
                zone(ZoneKind::Tomb)
            }
            RoomUse::Barracks => {
                self.place_building(BuildingKind::ArcheryTarget, r.a);
                zone(ZoneKind::Barracks)
            }
            RoomUse::Office => zone(ZoneKind::Office),
            RoomUse::Jail => zone(ZoneKind::Jail),
        };
        if let Some(k) = z {
            self.add_zone(r.a, r.b, k);
        }
        Self::note(o, tick, format!("Fitted out the {:?}.", r.kind).to_lowercase().replace("fitted", "Fitted"));
    }

    /// Keep stocks up with work orders, scaled to the fortress.
    fn ov_orders(&mut self, o: &Overseer) {
        if self.tick % 600 >= 150 {
            return;
        }
        let pop = self.alive_count() as u32;
        let mut orders: Vec<(Recipe, u32)> = vec![
            (Recipe::Brew, 20 + pop * 2),
            (Recipe::Cook, 10 + pop),
            (Recipe::WoodBed, 2),
            (Recipe::WoodTable, 2),
            (Recipe::WoodChair, 2),
            (Recipe::WoodDoor, 2),
            (Recipe::Barrel, 4),
            (Recipe::Bin, 3),
            (Recipe::Blocks, if o.style == Style::Builder { 40 } else { 10 }),
            (Recipe::StoneCraft, 8),
            (Recipe::Bucket, 1),
            (Recipe::Splint, 2),
            (Recipe::Crutch, 1),
            (Recipe::SpinThread, 4),
            (Recipe::Weave, 3),
            (Recipe::ClothShirt, 2),
            (Recipe::ClothTrousers, 2),
            (Recipe::Mechanism, if o.style == Style::Reckless { 0 } else { 5 }),
            (Recipe::WoodCage, if o.style == Style::Reckless { 0 } else { 4 }),
            (Recipe::Charcoal, 6),
            (Recipe::SmeltOre, 6),
        ];
        if o.style != Style::Reckless {
            orders.push((Recipe::WoodCoffin, 2));
        }
        if o.style != Style::Reckless {
            orders.push((Recipe::WoodCrossbow, 3));
            orders.push((Recipe::Bolts, 6));
        }
        if self.buildings.values().any(|b| b.kind == BuildingKind::Forge && b.built) {
            orders.push((Recipe::ForgeAxe, 2));
            orders.push((Recipe::ForgeHelm, 2));
            orders.push((Recipe::ForgeBreastplate, 1));
            orders.push((Recipe::ForgeShield, 1));
        }
        self.ambition_orders(o, &mut orders);
        // Only what some workshop can make (the rest would just complain).
        for (r, keep) in orders {
            if keep > 0 && self.buildings.values().any(|b| b.built && b.kind.recipes().contains(&r)) {
                self.add_work_order(r, keep);
            }
        }
        // Soldiers hunt game for the butcher.
        self.hunt = o.style != Style::Reckless && self.buildings.values().any(|b| b.built && b.kind == BuildingKind::Butcher);
    }

    /// Fill the posts with whoever fits best and isn't busy with another.
    fn ov_posts(&mut self) {
        let pop = self.alive_count();
        for (p, min) in [
            (Position::Manager, 0),
            (Position::Bookkeeper, 8),
            (Position::Broker, 8),
            (Position::ChiefMedic, 10),
            (Position::Commander, 10),
            (Position::Sheriff, 12),
        ] {
            if pop < min || self.has_post(p) {
                continue;
            }
            let pick = self.candidates(p).into_iter().find(|&id| self.position_of(id).is_none() && self.squad_of(id).is_none());
            if let Some(id) = pick {
                self.appoint(p, Some(id));
            }
        }
    }

    /// Keep a share of the adults under arms, the best fighters first.
    fn ov_army(&mut self, o: &mut Overseer) {
        let (share, from) = o.style.army();
        let adults = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        let mut want = if adults >= from { ((adults as f64 * share).ceil() as usize).max(3) } else { 0 };
        if o.ambition == Some(Ambition::Muster) {
            want = want.max(self.muster_target(o));
        }
        // Even the reckless take up arms once they've been attacked.
        if o.style == Style::Reckless && self.siege_size > 0 {
            want = 2;
        }
        if want == 0 {
            return;
        }
        if self.squads.is_empty() {
            self.new_squad();
            let name = self.squads[0].name.clone();
            Self::note(o, self.tick, format!("Raised a squad, {}.", name));
        }
        // Crossbows from the carpenter arm a squad soonest; metal comes later.
        let bows = self.stock_of(ItemKind::Crossbow) + self.dwarves.iter().filter(|d| d.alive && d.soldier && self.holds_crossbow(d)).count();
        let weapon = if bows >= 2 && o.style != Style::Warlord { super::military::UniformWeapon::Crossbow } else { super::military::UniformWeapon::Best };
        self.squads[0].weapon = weapon;
        for s in 1..self.squads.len() {
            self.squads[s].weapon = super::military::UniformWeapon::Best;
        }
        let have: usize = self.squads.iter().map(|s| s.members.len()).sum();
        if have >= want {
            return;
        }
        let mut cands: Vec<(u32, u32)> = self
            .dwarves
            .iter()
            .filter(|d| d.alive && !d.is_child() && !d.insane && d.noble.is_none() && self.position_of(d.id).is_none() && self.squad_of(d.id).is_none())
            .map(|d| (d.fight_xp, d.id))
            .collect();
        cands.sort_by(|a, b| b.cmp(a));
        for (_, id) in cands.into_iter().take(want - have) {
            // Into whichever squad is smallest.
            let s = (0..self.squads.len()).min_by_key(|&i| self.squads[i].members.len()).unwrap_or(0);
            self.set_squad_member(s, id, true);
        }
    }

    /// Sound the alarm while enemies are at the gates, stand down when
    /// they're gone. (A horror far down in the caverns doesn't count.)
    fn ov_alert(&mut self, o: &mut Overseer) {
        let (top, base) = (o.top.unwrap(), o.base.unwrap());
        let near_fort = |p: Pos| {
            let close = |c: Pos| (p.x - c.x).abs() <= 30 && (p.y - c.y).abs() <= 30;
            (close(top) || close(base)) && p.z >= base.z - 2 && p.z <= top.z + 2
        };
        // An army on the map is danger wherever it is; a lone monster only
        // when it comes near.
        let danger = self.creatures.values().any(|c| {
            c.alive && c.hostile() && !c.sneaking && ((c.siege && !c.legendary()) || ((c.siege || c.legendary()) && near_fort(c.pos)))
        });
        if danger != self.alert {
            self.alert = danger;
        }
        if danger && self.creatures.values().any(|c| c.alive && c.siege && !c.legendary() && c.hostile()) {
            o.last_siege = self.tick;
        }
        if o.style != Style::Reckless && self.alert_burrow.is_some() && danger != self.civ_alert {
            let _ = self.toggle_civ_alert();
        }
        // Bar the gate when they're close (not before, so those outside can
        // still get in).
        let close = self.creatures.values().any(|c| {
            c.alive && c.hostile() && !c.sneaking && (c.siege || c.legendary()) && {
                let p = c.pos;
                let by = |q: Pos, r: i32| (p.x - q.x).abs() <= r && (p.y - q.y).abs() <= r && (p.z - q.z).abs() <= 1;
                by(top, 10) || by(base, 12)
            }
        });
        let gate: Vec<u32> = self
            .buildings
            .values()
            .filter(|b| {
                b.kind == BuildingKind::Door
                    && b.built
                    && b.locked != close
                    && (((b.pos.x - base.x).abs() <= 1 && (b.pos.y - base.y).abs() <= 1 && b.pos.z == base.z) || b.pos == top.offset(-3, 0, 0))
            })
            .map(|b| b.id)
            .collect();
        for id in gate {
            self.toggle_lock(id);
        }
        // Raise the drawbridge against an army, once everyone's inside or
        // when they're close; not for days on end without one at the gate.
        if o.did("linked") {
            let inside = self.dwarves.iter().filter(|d| d.alive && !d.soldier).all(|d| d.pos.z <= base.z && !self.world.outside(d.pos));
            let army = |near: bool| {
                self.creatures.values().any(|c| {
                    c.alive && c.siege && !c.legendary() && c.hostile() && (!near || ((c.pos.x - top.x).abs() <= 25 && (c.pos.y - top.y).abs() <= 25))
                })
            };
            let (any, at_gate) = (army(false), army(true));
            let long = self.tick > o.raised_at + DAY * 4;
            let want = any && (at_gate || inside) && !(long && !at_gate && o.raised_at > 0);
            let bridges = self.gate_bridges(base);
            let raised = bridges.iter().any(|id| self.buildings.get(id).map_or(false, |b| b.raised));
            if let Some(lever) = self.gate_lever(base) {
                if raised != want && !self.buildings[&lever].pull {
                    self.request_pull(lever);
                }
                if want && !raised {
                    o.raised_at = self.tick;
                } else if !want && !raised {
                    o.raised_at = 0;
                }
            }
        }
        // Soldiers hold the corridor behind the gate, unless they're strong
        // enough (and bold enough) to go out and meet them.
        if !self.squads.is_empty() {
            let strength = self.squad_strength(&self.squads[0].members.clone());
            let charge = o.style == Style::Warlord && strength >= 15;
            let holding = self.squads[0].order == super::SquadOrder::Station;
            for s in 0..self.squads.len() {
                let holding = self.squads[s].order == super::SquadOrder::Station;
                if danger && !charge && !holding {
                    self.order_station(s, base.offset(4 + s as i32, 0, 0));
                } else if !danger && holding {
                    self.order_train(s);
                }
            }
            let _ = holding;
        }
    }

    fn gate_lever(&self, base: Pos) -> Option<u32> {
        self.buildings.values().find(|b| b.kind == BuildingKind::Lever && b.built && b.pos == base.offset(3, 1, 0)).map(|b| b.id)
    }

    fn gate_bridges(&self, base: Pos) -> Vec<u32> {
        self.buildings.values().filter(|b| b.kind == BuildingKind::Bridge && b.pos.z == base.z && b.pos.x == base.x + 2 && (b.pos.y - base.y).abs() <= 1).map(|b| b.id).collect()
    }

    /// The gate at the foot of the stair, and a corridor of cage traps
    /// behind it.
    fn ov_defences(&mut self, o: &mut Overseer) {
        if o.style == Style::Reckless {
            return;
        }
        let base = o.base.unwrap();
        if !o.did("gate") && self.dug_share(base.offset(1, 0, 0), base.offset(6, 1, 0)) >= 1.0 {
            let a = self.place_building(BuildingKind::Door, base.offset(1, 0, 0));
            let b = self.place_building(BuildingKind::Door, base.offset(1, 1, 0));
            if a || b {
                o.done.push("gate".into());
                Self::note(o, self.tick, "Hung a gate at the foot of the stair, to bar when enemies come.".into());
            }
        }
        // A drawbridge behind the doors, on a lever: raised, nothing gets in.
        if o.did("gate") && !o.did("bridge") && self.stock_of(ItemKind::Mechanism) >= 3 {
            let a = self.place_building(BuildingKind::Bridge, base.offset(2, 0, 0));
            let b = self.place_building(BuildingKind::Bridge, base.offset(2, 1, 0));
            let l = self.place_building(BuildingKind::Lever, base.offset(3, 1, 0));
            if a || b || l {
                o.done.push("bridge".into());
                Self::note(o, self.tick, "Began a drawbridge at the gate, worked by a lever inside.".into());
            }
        }
        if o.did("bridge") && !o.did("linked") {
            let lever = self.gate_lever(base);
            let bridges = self.gate_bridges(base);
            let built = bridges.iter().all(|id| self.buildings.get(id).map_or(false, |b| b.built));
            if let (Some(lever), true, false) = (lever, built, bridges.is_empty()) {
                let linked = bridges.iter().filter(|&&id| self.link_lever(lever, id).is_ok()).count();
                if linked == bridges.len() {
                    o.done.push("linked".into());
                    Self::note(o, self.tick, "Linked the lever to the drawbridge. The gate can be shut fast.".into());
                }
            }
        }
        if o.did("gate") && !o.did("traps") && self.stock_of(ItemKind::Mechanism) >= 2 && self.stock_of(ItemKind::Cage) >= 2 {
            let mut n = 0;
            for dx in 4..8 {
                for dy in 0..2 {
                    if self.place_building(BuildingKind::CageTrap, base.offset(dx, dy, 0)) {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                o.done.push("traps".into());
                Self::note(o, self.tick, format!("Set {} cage traps in the entrance corridor.", n));
            }
        }
    }

    /// See that someone does each job the workshops and fields need, the
    /// most skilled first.
    fn ov_labors(&mut self, o: &Overseer) {
        let pop = self.alive_count();
        let has = |g: &Game, k: BuildingKind| g.buildings.values().any(|b| b.built && b.kind == k);
        let mut need: Vec<(Labor, usize)> = vec![
            (Labor::Mining, 2 + pop / 8),
            (Labor::Woodcutting, 1 + pop / 15),
            (Labor::Farming, 2 + pop / 6),
            (Labor::Building, 2),
            (Labor::Carpentry, 1),
            (Labor::Masonry, 1),
            (Labor::Brewing, 1),
            (Labor::Cooking, 1),
        ];
        for (k, l) in [
            (BuildingKind::Mechanic, Labor::Mechanics),
            (BuildingKind::Craftsdwarf, Labor::Crafting),
            (BuildingKind::Butcher, Labor::Butchery),
            (BuildingKind::Smelter, Labor::Smelting),
            (BuildingKind::WoodFurnace, Labor::Smelting),
            (BuildingKind::Forge, Labor::Smithing),
            (BuildingKind::Loom, Labor::Clothesmaking),
            (BuildingKind::Tannery, Labor::Leatherworking),
        ] {
            if has(self, k) {
                need.push((l, 1));
            }
        }
        if self.zones.values().any(|z| z.kind == ZoneKind::Hospital) {
            need.push((Labor::Medicine, 1 + pop / 15));
        }
        if o.style == Style::Builder {
            need.push((Labor::Engraving, 2));
        }
        for (labor, want) in need {
            let have = self.dwarves.iter().filter(|d| d.alive && !d.is_child() && !d.soldier && d.has(labor)).count();
            if have >= want {
                continue;
            }
            let mut cands: Vec<(u32, usize, u32)> = self
                .dwarves
                .iter()
                .filter(|d| d.alive && !d.is_child() && !d.soldier && !d.insane && !d.has(labor))
                .map(|d| (d.skills[labor.idx()], LABORS.iter().filter(|l| d.has(**l)).count(), d.id))
                .collect();
            // The most skilled, then whoever has least to do.
            cands.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            for (_, _, id) in cands.into_iter().take(want - have) {
                self.set_labor(id, labor, true);
            }
        }
    }

    /// Once a day: the bigger decisions.
    fn ov_daily(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let base = o.base.unwrap();
        let e = self.embark;
        self.ov_labors(o);
        // Keep the wood coming: chop further out when it runs low.
        if self.stock_of(ItemKind::Log) < 6 {
            let chopping = self.designations.iter().filter(|p| self.world.t(**p).desig == Desig::Chop).count();
            if chopping < 5 {
                let k = o.done.iter().filter(|d| d.starts_with("chop")).count() as i32;
                let r = 22 + k * 8;
                self.designate(e.offset(-r, -r, -2), e.offset(r, r, 2), Desig::Chop);
                o.done.push(format!("chop{}", k));
            }
        }
        self.ov_defences(o);
        // More fields as the fortress grows.
        let farm: usize = self.zones.values().filter(|z| z.kind == ZoneKind::Farm).map(|z| z.tiles.len()).sum();
        if farm < self.alive_count() * 4 {
            let k = o.done.iter().filter(|d| d.starts_with("farm")).count() as i32;
            let (dx, dy) = [(-20, -8), (12, -8), (-20, 6), (12, 6), (-8, -18), (2, -18), (-8, 14), (2, 14)][(k % 8) as usize];
            self.add_zone(e.offset(dx, dy, -1), e.offset(dx + 7, dy + 5, 1), ZoneKind::Farm);
            o.done.push(format!("farm{}", k));
        }
        // A depot for the caravans, and trade when they come.
        if !o.did("depot") {
            if let Some(p) = self.ov_spot(BuildingKind::Depot, e.offset(-6, 0, 0), 16) {
                self.place_building(BuildingKind::Depot, p);
                o.done.push("depot".into());
            }
        }
        self.ov_trade(o);
        // A safe burrow over the main level for when enemies come.
        if o.style != Style::Reckless && !o.did("burrow") && o.rooms.len() >= 4 {
            if let Some(i) = self.new_burrow() {
                let r = 12 + o.slots * 8;
                self.paint_burrow(i, base.offset(-2, -10, 0), base.offset(r, 10, 0), true);
                self.set_alert_burrow(i);
                o.done.push("burrow".into());
                Self::note(o, tick, "Marked out a burrow to shelter in when enemies come.".into());
            }
        }
        // Mine what's been found: ore and gems in sight of the halls.
        self.ov_mine(o, 12, 20);
        // Each style's habits, whatever its ambition.
        match o.style {
            Style::Delver => self.ov_shaft(o),
            Style::Builder => self.ov_detail(o, self.tick > YEAR / 2),
            _ => {}
        }
        // Builders (and the steady, when rich) raise a monument.
        let rich = self.wealth() >= if o.style == Style::Builder { 15_000 } else { 40_000 };
        if rich && matches!(o.style, Style::Builder | Style::Steady) && !o.did("monument") {
            if let Some(p) = self.ov_spot(BuildingKind::Monument, e.offset(0, 6, 0), 16) {
                self.place_building(BuildingKind::Monument, p);
                o.done.push("monument".into());
                Self::note(o, tick, "Began a great monument.".into());
            }
        }
        // Warlords (and now and then others with a strong squad) send expeditions.
        if o.style == Style::Warlord {
            self.ov_venture(o, 0.55, true);
        } else {
            self.ov_venture(o, 0.7, false);
        }
        // And whatever it has its heart set on.
        self.ov_ambition(o);
        let _ = tick;
    }

    /// Haul goods to the depot when merchants come, and swap them for what
    /// the fortress needs.
    fn ov_trade(&mut self, o: &mut Overseer) {
        let arrived = self.caravan.as_ref().map_or(false, |c| c.arrived);
        if !arrived {
            if self.caravan.is_none() && self.items.values().any(|it| it.to_depot) {
                self.mark_for_trade(false);
            }
            return;
        }
        self.mark_for_trade(true);
        let Some(depot) = self.depot().map(|b| b.tiles()) else { return };
        let ours: Vec<u32> = self.items.values().filter(|it| it.to_depot && depot.contains(&it.pos) && it.carried_by.is_none()).map(|it| it.id).collect();
        let offer: u32 = ours.iter().filter_map(|id| self.items.get(id)).map(|it| it.value()).sum();
        if offer == 0 {
            return;
        }
        // Food and drink first, then metal, cloth and the rest.
        let rank = |k: ItemKind| match k {
            ItemKind::Drink | ItemKind::Meal | ItemKind::Meat | ItemKind::Plant | ItemKind::Fish => 0,
            ItemKind::Bar => 1,
            ItemKind::Cloth | ItemKind::Leather | ItemKind::Thread => 2,
            _ => 3,
        };
        let mut goods: Vec<(u8, u32, u32)> = self.caravan.as_ref().unwrap().goods.iter().map(|g| (rank(g.kind), g.value(), g.id)).collect();
        goods.sort();
        let budget = offer * 100 / (100 + self.trade_margin());
        let mut spent = 0;
        let mut theirs = Vec::new();
        for (_, v, id) in goods {
            if spent + v <= budget {
                spent += v;
                theirs.push(id);
            }
        }
        if theirs.is_empty() {
            return;
        }
        if let Ok(m) = self.trade(&ours, &theirs) {
            self.mark_for_trade(false);
            Self::note(o, self.tick, m);
        }
    }
}

/// How one autoplayed fortress turned out.
#[derive(Clone, Debug)]
pub struct Trial {
    pub seed: u64,
    pub style: Style,
    pub fort: String,
    pub years: f64,
    pub alive: usize,
    pub peak_pop: usize,
    pub peak_wealth: u32,
    pub milestones: Vec<String>,
    pub heroes: Vec<String>,
    /// The ambitions it went through, in order.
    pub ambitions: Vec<String>,
    /// The chronicle's titles, year by year.
    #[allow(dead_code)]
    pub chronicles: Vec<String>,
    /// Ruin, Glory or Endured.
    pub verdict: &'static str,
    /// The last word on it: what ended it, or its greatest moment.
    pub epitaph: String,
}

/// Play one fortress with `style` for up to `years`, from a fresh world.
pub fn run_trial(seed: u64, style: Style, years: u64) -> Trial {
    let mut g = Game::new(seed);
    g.set_overseer(Some(style));
    let mut last_death = String::new();
    let end = years * YEAR;
    let mut seen = 0u64;
    let story = std::env::var("TRIAL_DEATHS").is_ok();
    while g.tick < end && g.alive_count() > 0 {
        g.step();
        if story && g.tick % 50 == 0 {
            for l in g.log.iter().filter(|l| l.tick > seen) {
                let t = &l.text;
                if l.color == (255, 60, 60) || t.contains("arrived!") || t.contains("has come") || t.contains("fleeing") || t.contains("bashes down") || t.contains("Lever") || t.contains("raised") || t.contains("lowered") || t.contains("caught") {
                    eprintln!("    d{:>3} {}", l.tick / DAY, t);
                }
            }
            seen = g.tick;
        }
        if g.tick % DAY == 0 {
            if let Some(l) = g.log.iter().rev().find(|l| l.color == (255, 60, 60)) {
                last_death = l.text.clone();
            }
        }
        if g.tick % (SEASON / 3) == 0 && std::env::var("TRIAL_LOG").is_ok() {
            let food = g.items.values().filter(|it| it.is_food() && it.kind != ItemKind::Drink && it.kind != ItemKind::Seed).count();
            let drink = g.stock_of(ItemKind::Drink);
            let farm: usize = g.zones.values().filter(|z| z.kind == ZoneKind::Farm).map(|z| z.tiles.len()).sum();
            let hostile = g.creatures.values().filter(|c| c.alive && c.hostile()).count();
            eprintln!(
                "  [{} {}] day {:>4} pop {:>2} food {:>3} drink {:>3} farm {:>3} alert {} civ {} hostile {} rooms {} desig {}",
                style.name(),
                seed,
                g.tick / DAY,
                g.alive_count(),
                food,
                drink,
                farm,
                g.alert,
                g.civ_alert,
                hostile,
                g.overseer.as_ref().map_or(0, |o| o.rooms.len()),
                g.designations.len()
            );
            eprintln!(
                "      done {:?} soldiers {} doors {} traps {} raised {} squad {:?} | boulders {} logs {} mech {} cages {} miners {} shops {:?}",
                g.overseer.as_ref().map(|o| o.done.iter().filter(|d| !d.starts_with("farm")).cloned().collect::<Vec<_>>()),
                g.dwarves.iter().filter(|d| d.alive && d.soldier).count(),
                g.buildings.values().filter(|b| b.kind == BuildingKind::Door && b.built).count(),
                g.buildings.values().filter(|b| b.kind.is_trap() && b.built).count(),
                g.buildings.values().filter(|b| b.kind == BuildingKind::Bridge && b.raised).count(),
                g.squads.first().map(|s| s.order.clone()),
                g.stock_of(ItemKind::Boulder),
                g.stock_of(ItemKind::Log),
                g.stock_of(ItemKind::Mechanism),
                g.stock_of(ItemKind::Cage),
                g.dwarves.iter().filter(|d| d.alive && d.has(Labor::Mining)).count(),
                g.buildings.values().filter(|b| b.kind.is_workshop()).map(|b| (b.kind.letter(), b.built, b.orders.len())).collect::<Vec<_>>()
            );
        }
    }
    if let Some(l) = g.log.iter().rev().find(|l| l.color == (255, 60, 60)) {
        last_death = l.text.clone();
    }
    if std::env::var("TRIAL_TAIL").is_ok() {
        for l in g.log.iter().rev().take(60).collect::<Vec<_>>().into_iter().rev() {
            eprintln!("    t{:>6} {}", l.tick, l.text);
        }
    }
    let o = g.overseer.clone().unwrap_or_else(|| Overseer::new(style));
    let alive = g.alive_count();
    let heroes: Vec<String> = g.dwarves.iter().filter(|d| d.epithet.is_some()).map(|d| d.called()).collect();
    let glory = g.milestones.len() >= 6 || o.peak_wealth >= 60_000 || g.monument.is_some();
    let verdict = if alive == 0 {
        "Ruin"
    } else if glory {
        "Glory"
    } else {
        "Endured"
    };
    let epitaph = if alive == 0 {
        format!("Fell in year {}: {}", g.tick / YEAR + 1, last_death)
    } else if let Some(m) = &g.monument {
        format!("Raised {}", m)
    } else if let Some(m) = g.milestones.last() {
        m.1.clone()
    } else {
        "Quiet years".into()
    };
    Trial {
        seed,
        style,
        fort: g.fortress_name.clone(),
        years: g.tick as f64 / YEAR as f64,
        alive,
        peak_pop: o.peak_pop,
        peak_wealth: o.peak_wealth.max(g.wealth()),
        milestones: g.milestones.iter().map(|m| m.1.clone()).collect(),
        heroes,
        ambitions: o.ambitions.iter().map(|a| a.1.name().to_string()).collect(),
        chronicles: g.chronicles.iter().map(|c| c.title.clone()).collect(),
        verdict,
        epitaph,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overseer_digs_in_and_builds_a_fortress() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        for _ in 0..(SEASON + SEASON / 2) {
            g.step();
        }
        let o = g.overseer.as_ref().expect("the overseer quit");
        assert!(o.base.is_some(), "no stair dug");
        assert!(o.rooms.len() >= 4, "only {} rooms", o.rooms.len());
        assert!(o.rooms.iter().filter(|r| r.stage == 1).count() >= 2, "rooms never fitted out: {:?}", o.rooms.iter().map(|r| (r.kind, r.stage)).collect::<Vec<_>>());
        assert!(g.has_post(Position::Manager));
        assert!(!g.work_orders.is_empty());
        let workshops = g.buildings.values().filter(|b| b.kind.is_workshop() && b.built).count();
        assert!(workshops >= 4, "only {} workshops built", workshops);
        let base = o.base.unwrap();
        assert!(g.zones.values().any(|z| z.tiles.iter().any(|p| p.z == base.z)), "nothing zoned below");
    }

    #[test]
    #[ignore]
    fn style_trials() {
        let years: u64 = std::env::var("YEARS").ok().and_then(|s| s.parse().ok()).unwrap_or(2);
        let handles: Vec<_> = Style::ALL
            .iter()
            .enumerate()
            .filter(|(_, s)| std::env::var("ONLY").map_or(true, |o| o.eq_ignore_ascii_case(s.name())))
            .map(|(i, &s)| std::thread::spawn(move || run_trial(100 + i as u64, s, years)))
            .collect();
        for h in handles {
            let t = h.join().unwrap();
            eprintln!(
                "{:<9} seed {:>3} {:<14} {:>4.1}y alive {:>2} peak {:>2} ☼{:>7} {:<8} {} | {:?} | {:?}",
                t.style.name(), t.seed, t.fort, t.years, t.alive, t.peak_pop, t.peak_wealth, t.verdict, t.epitaph, t.chronicles, t.heroes
            );
        }
    }

    #[test]
    #[ignore]
    fn overseer_speed() {
        let mut g = Game::new(5);
        let style = std::env::var("STYLE").ok().and_then(|s| Style::parse(&s)).unwrap_or(Style::Steady);
        g.set_overseer(Some(style));
        let mut last = std::time::Instant::now();
        for month in 0..9 {
            for _ in 0..MONTH {
                g.step();
            }
            eprintln!(
                "month {} {:?}: pop {} desig {} (smooth/engrave {}) constructions {} items {} creatures {}",
                month,
                last.elapsed(),
                g.alive_count(),
                g.designations.len(),
                g.designations.iter().filter(|p| matches!(g.world.t(**p).desig, Desig::Smooth | Desig::Engrave)).count(),
                g.constructions.len(),
                g.items.len(),
                g.creatures.len()
            );
            let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
            for p in &g.designations {
                *kinds.entry(format!("{:?}", g.world.t(*p).desig)).or_default() += 1;
            }
            let base = g.overseer.as_ref().and_then(|o| o.base);
            let zs: std::collections::BTreeMap<i32, usize> = g.designations.iter().fold(Default::default(), |mut m, p| {
                *m.entry(p.z - base.map_or(0, |b| b.z)).or_default() += 1;
                m
            });
            eprintln!("    kinds {:?} by z-from-base {:?}", kinds, zs);
            last = std::time::Instant::now();
        }
    }

    #[test]
    #[ignore]
    fn one_trial() {
        let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let style = std::env::var("STYLE").ok().and_then(|s| Style::parse(&s)).unwrap_or(Style::Steady);
        let years: u64 = std::env::var("YEARS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let t = run_trial(seed, style, years);
        eprintln!("{} {} {:.1}y {} | {}", t.style.name(), t.seed, t.years, t.verdict, t.epitaph);
    }

    #[test]
    fn styles_parse_and_trials_report() {
        assert_eq!(Style::parse("warlord"), Some(Style::Warlord));
        assert_eq!(Style::parse("nope"), None);
        let t = run_trial(7, Style::Reckless, 0);
        assert_eq!(t.verdict, "Endured");
        assert!(t.alive > 0);
    }
}
