//! The overseer: an AI that plays the fortress through the same commands a
//! player has (designations, buildings, zones, work orders, squads, posts,
//! trade, expeditions), in one of several styles. Turn it on with `A` in a
//! fortress, watch one from the title screen, or run many from the command
//! line to see which styles end in ruin and which in glory.

use super::*;
use crate::buildings::Recipe;
use super::ambitions::Ambition;
use super::layout::{Layout, Shape};
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
    /// The Architect's grand work (see `game::layout`).
    Grand,
    /// A row of bedrooms off a hall, a bed and a door each.
    Bedrooms,
    /// A noble's rooms: bedroom, office and dining room (`architect`).
    Suite,
    /// A quiet room of desks and cabinets.
    Library,
    /// A small meeting room.
    Parlour,
    /// One chain's workshops with their stockpile (`maturity`).
    Industry,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub kind: RoomUse,
    pub a: Pos,
    pub b: Pos,
    /// 0 dug out (designated), 1 fitted out.
    pub stage: u8,
    /// The tiles it's cut from (empty: all of a..b).
    #[serde(default)]
    pub tiles: Vec<Pos>,
    #[serde(default)]
    pub shape: Option<Shape>,
    /// When it was laid out.
    #[serde(default)]
    pub since: u64,
    /// Doors onto its hall (rooms from a plan).
    #[serde(default)]
    pub doors: Vec<Pos>,
    /// Where its beds go (a row of bedrooms).
    #[serde(default)]
    pub beds: Vec<Pos>,
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
    /// Your nudges to each ambition (in `Ambition::ALL` order), -3..=3.
    #[serde(default)]
    pub nudges: Vec<i8>,
    /// How it lays out its halls.
    #[serde(default)]
    pub layout: Layout,
    /// The deepest level of halls it has opened.
    #[serde(default)]
    pub level: i32,
    /// Doors and room centres new tunnels can join.
    #[serde(default)]
    pub anchors: Vec<Pos>,
    /// Slots of the layout's plan already used or found blocked: (level, slot).
    #[serde(default)]
    pub plan_taken: Vec<(i32, u32)>,
    /// How far east the plan of halls runs (0: not drawn up yet).
    #[serde(default)]
    pub plan_len: i32,
    /// When the army now on the map first came (0: none).
    #[serde(default)]
    pub army_since: u64,
    /// Troubles it has seen to lately (so it doesn't fuss over them daily).
    #[serde(default)]
    pub fixes: Vec<(super::troubles::Lack, u64)>,
    /// Work orders it raised to meet a shortage: (recipe, keep, until).
    #[serde(default)]
    pub boosts: Vec<(Recipe, u32, u64)>,
    /// Who it is: a name and two traits (see `personality`).
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub traits: Vec<super::personality::Trait>,
    /// What it decided, newest last.
    pub notes: Vec<(u64, String)>,
    /// What sieges have taught it, and when (`lessons.rs`).
    #[serde(default)]
    pub lessons: Vec<(u64, super::lessons::Lesson)>,
    /// Shortages it is seeing to, each as a whole chain (`troubles`).
    #[serde(default)]
    pub worries: Vec<super::troubles::Worry>,
    /// Goods it means to buy from the next caravan (to break a circle).
    #[serde(default)]
    pub wanted: Vec<ItemKind>,
    /// Rooms it found nowhere to cut, and when (not asked for again soon).
    #[serde(default)]
    pub blocked: Vec<(RoomUse, u64)>,
    /// Since when its rooms have found nowhere to go (0: they do).
    #[serde(default)]
    pub stuck_since: u64,
    /// The watchdog's mark: rooms fitted and buildings built, and since
    /// when they've stayed the same.
    #[serde(default)]
    pub stall_mark: (usize, usize),
    #[serde(default)]
    pub stall_since: u64,
    /// How many times the watchdog has had to step in, and moved the stair.
    #[serde(default)]
    pub stalls: u32,
    #[serde(default)]
    pub resites: u32,
    /// How the inside of the fort grows up (`architect`, `maturity`).
    #[serde(default)]
    pub interior: super::architect::Interior,
    /// The eulogies: deaths are looked at from this tick on (0: from a day
    /// back), less those already remembered on that tick.
    #[serde(default)]
    pub eulogy_from: u64,
    #[serde(default)]
    pub eulogized: Vec<u32>,
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
            nudges: Vec::new(),
            layout: Layout::Orderly,
            level: 0,
            anchors: Vec::new(),
            plan_taken: Vec::new(),
            plan_len: 0,
            army_since: 0,
            fixes: Vec::new(),
            boosts: Vec::new(),
            name: String::new(),
            traits: Vec::new(),
            notes: Vec::new(),
            lessons: Vec::new(),
            worries: Vec::new(),
            wanted: Vec::new(),
            blocked: Vec::new(),
            stuck_since: 0,
            stall_mark: (0, 0),
            stall_since: 0,
            stalls: 0,
            resites: 0,
            interior: Default::default(),
            eulogy_from: 0,
            eulogized: Vec::new(),
        }
    }

    /// Whether it digs rooms anywhere round its halls, not just east of the
    /// gatehouse (once the watchdog has had to step in).
    pub(super) fn anywhere(&self) -> bool {
        self.stalls > 0
    }

    /// Whether it found nowhere for a room of `kind` lately (this season).
    pub(super) fn blocked_lately(&self, kind: RoomUse, tick: u64) -> bool {
        self.blocked.iter().any(|&(k, t)| k == kind && tick < t + SEASON)
    }

    pub fn nudge(&self, a: Ambition) -> i8 {
        Ambition::ALL.iter().position(|&x| x == a).and_then(|i| self.nudges.get(i).copied()).unwrap_or(0)
    }

    pub(super) fn did(&self, what: &str) -> bool {

        self.done.iter().any(|d| d == what)
    }
}

/// Where new fields go around the wagon (offsets of an 8 by 6 plot), each
/// tried once.
const FIELD_SPOTS: [(i32, i32); 8] = [(-20, -8), (12, -8), (-20, 6), (12, 6), (-8, -18), (2, -18), (-8, 14), (2, 14)];

/// Which of the field spots a "done" entry tells of: "farm{i}" for a field
/// laid out there, "farm{i}x" for a spot that took none (no soil, nothing
/// free or seen).
fn field_entry(d: &str) -> Option<usize> {
    d.strip_prefix("farm")?.trim_end_matches('x').parse().ok()
}

impl Game {
    /// Lay out a field at the next spot not yet tried; whether one was made.
    /// Once all eight are used or refused there are no more to try. (Saves
    /// from before kept a "farm{k}" for every day's try, hundreds of them:
    /// those past the eight spots are dropped.)
    pub(super) fn ov_more_fields(&mut self, o: &mut Overseer) -> bool {
        o.done.retain(|d| field_entry(d).map_or(true, |i| i < FIELD_SPOTS.len()));
        let Some(i) = (0..FIELD_SPOTS.len()).find(|&i| !o.done.iter().any(|d| field_entry(d) == Some(i))) else { return false };
        let (dx, dy) = FIELD_SPOTS[i];
        let e = self.embark;
        let made = self.add_zone(e.offset(dx, dy, -1), e.offset(dx + 7, dy + 5, 1), ZoneKind::Farm) > 0;
        o.done.push(if made { format!("farm{}", i) } else { format!("farm{}x", i) });
        made
    }
}

/// What a room is called in its notes.
pub(super) fn room_name(k: RoomUse) -> &'static str {
    match k {
        RoomUse::Workshops => "workshop hall",
        RoomUse::Stockpile => "storeroom",
        RoomUse::Dining => "dining hall",
        RoomUse::Dormitory => "dormitory",
        RoomUse::Meeting => "meeting hall",
        RoomUse::Tavern => "tavern",
        RoomUse::Temple => "temple",
        RoomUse::Hospital => "hospital",
        RoomUse::Tomb => "tomb",
        RoomUse::Barracks => "barracks",
        RoomUse::Office => "office",
        RoomUse::Jail => "jail",
        RoomUse::Grand => "grand hall",
        RoomUse::Bedrooms => "row of bedrooms",
        RoomUse::Suite => "noble's suite",
        RoomUse::Library => "library",
        RoomUse::Parlour => "parlour",
        RoomUse::Industry => "workshop quarter",
    }
}

/// The order rooms are dug in, by style.
pub(super) fn plan(style: Style) -> Vec<RoomUse> {
    use RoomUse::*;
    match style {
        // (A hospital right after the first beds: it must be ready before the first siege.)
        Style::Builder => vec![Workshops, Stockpile, Dining, Dormitory, Hospital, Meeting, Workshops, Tavern, Temple, Tomb, Office, Stockpile, Dormitory, Barracks, Jail],
        Style::Warlord => vec![Workshops, Stockpile, Barracks, Dining, Dormitory, Hospital, Workshops, Meeting, Tomb, Office, Stockpile, Dormitory, Tavern, Jail, Temple],
        Style::Delver => vec![Workshops, Stockpile, Dining, Dormitory, Hospital, Workshops, Stockpile, Meeting, Tomb, Barracks, Office, Dormitory, Tavern, Temple, Jail],
        // (Reckless has no hospital: the dormitory's beds will do.)
        Style::Reckless => vec![Workshops, Stockpile, Dining, Dormitory, Workshops, Tavern, Meeting, Stockpile, Dormitory, Temple],
        Style::Steady => vec![Workshops, Stockpile, Dining, Dormitory, Hospital, Meeting, Workshops, Tomb, Barracks, Tavern, Office, Stockpile, Dormitory, Temple, Jail],
    }
}

/// How far round the foot of the stair the quarry looks for stone (west,
/// east, and north or south: the halls spread east of the stair), and the
/// longest tunnel through soil it cuts to reach some.
const QUARRY_DX0: i32 = 40;
const QUARRY_DX1: i32 = 80;
const QUARRY_DY: i32 = 45;
const QUARRY_TUNNEL: i32 = 30;

const WORKSHOPS: [BuildingKind; 12] = [
    BuildingKind::Carpenter,
    BuildingKind::Mason,
    // Mechanisms early: the gate's drawbridge and traps need them before the first siege.
    BuildingKind::Mechanic,
    BuildingKind::Still,
    BuildingKind::Kitchen,
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
                if o.name.is_empty() {
                    self.roll_personality(&mut o);
                }
                let who = o.title();
                self.overseer = Some(o);
                self.msg(&format!("The overseer {} takes charge ({}: {}).", who, s.name(), s.blurb()), (150, 220, 255));
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
        // The very same words again so soon say nothing new: once a half
        // season at most (what still holds says "still", see `troubles`).
        if o.notes.iter().any(|n| n.1 == text && tick < n.0 + SEASON / 2) {
            return;
        }
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
        // The dead and the offal go outside, where they won't fill the halls with miasma.
        self.add_zone(e.offset(9, 6, -1), e.offset(12, 10, 1), ZoneKind::Stockpile(StockFilter::Refuse));
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
        let Some((s, depth)) = self.stair_site(o.style, None) else {
            self.overseer = None;
            return;
        };
        let base = s.offset(0, 0, -depth);
        if base.z < 2 {
            return;
        }
        self.designate(s, base, Desig::StairUD);
        // The gatehouse: the corridor every way in passes through.
        self.designate(base.offset(1, 0, 0), base.offset(14, 1, 0), Desig::Dig);
        o.top = Some(s);
        o.base = Some(base);
        o.slots = 0;
        o.layout = self.pick_layout(o);
        Self::note(o, tick, format!("It means to lay the fortress out as {}.", o.layout.name()));
        if let Some(m) = self.candidates(Position::Manager).first().copied() {
            self.appoint(Position::Manager, Some(m));
        }
        Self::note(o, tick, format!("Struck the earth: a stair down {} levels, farms and a stockpile by the wagon.", depth));
    }

    /// Where the stair goes down from the surface, and how deep: on open
    /// ground near the wagon, far from water, with rock for the gatehouse
    /// and room for the halls east of it. With `avoid` (a stair whose halls
    /// came to nothing), only somewhere away from it, and no making do with
    /// a site that fails the gatehouse's checks.
    pub(super) fn stair_site(&self, style: Style, avoid: Option<Pos>) -> Option<(Pos, i32)> {
        let e = self.embark;
        let depth = if style == Style::Delver { 4 } else { 3 };
        let open = |g: &Game, p: Pos| g.world.get(p).map_or(false, |t| t.kind == TileKind::Floor && t.building == 0 && t.zone == 0 && t.water == 0);
        // Somewhere water can't pour in: as far from any as can be (a river
        // beside the stair floods the whole fort), high above the waterline,
        // with dry rock where the gatehouse will run.
        // A heavy aquifer floods whatever is dug through it: the stair stops
        // short of it (the halls above it), or the site won't do.
        let reach = |g: &Game, p: Pos| -> i32 {
            if g.world.aquifer < 2 {
                return depth;
            }
            (1..=depth).take_while(|&k| g.world.get(p.offset(0, 0, -k)).map_or(false, |t| !t.aquifer) && (-1..=1).all(|dx| (-1..=1).all(|dy| g.world.get(p.offset(dx, dy, -k)).map_or(true, |t| !t.aquifer)))).last().unwrap_or(0)
        };
        let score_at = |g: &Game, p: Pos, depth: i32| -> Option<i32> {
            let w = &g.world;
            let base = p.offset(0, 0, -depth);
            let gate_dry = (0..=14).all(|dx| (0..=1).all(|dy| w.get(base.offset(dx, dy, 0)).map_or(false, |t| t.water == 0 && !t.aquifer)));
            // Under rock all the way: a gatehouse that comes out under the sky is no gate.
            let roofed = (0..=20).all(|dx| (-1..=2).all(|dy| {
                let q = base.offset(dx, dy, 0);
                !w.in_bounds(q) || w.surface_z(q.x, q.y) >= base.z + 2
            }));
            // And solid rock where it's cut, and above and below it: a cave
            // or a gully across the corridor (or in its walls) is a gap no
            // gate closes, and a pocket of water over it floods it.
            let solid = (-1..=16).all(|dx| (-1..=2).all(|dy| {
                let q = base.offset(dx, dy, 0);
                let rock = dx <= 0 && dy == 0 || w.get(q).map_or(false, |t| t.kind == TileKind::Wall && t.mat != Mat::Bedrock);
                rock && [q.up(), q.down()].iter().all(|&r| w.get(r).map_or(false, |t| t.kind.solid() && t.water == 0))
            }));
            // The stair, too, goes down through rock all the way: not
            // through a cave or a cavern (what lives there would come up it).
            let shaft = (1..=depth).all(|k| (-1..=1).all(|dx| (-1..=1).all(|dy| w.get(p.offset(dx, dy, -k)).map_or(false, |t| t.kind.solid()))));
            if !gate_dry || !roofed || !solid || !shaft || base.z < 2 {
                return None;
            }
            let far = (1..=8)
                .find(|&r| (-r..=r).any(|dy| (-r..=r).any(|dx| (dx as i32).abs().max((dy as i32).abs()) == r && (-1..=1).any(|dz| w.get(p.offset(dx, dy, dz)).map_or(false, |t| t.water > 0)))))
                .unwrap_or(9);
            let dist = (p.x - e.x).abs().max((p.y - e.y).abs());
            // Room for the halls: they run east from the end of the
            // gatehouse, so a stair by the map's east edge (or its north or
            // south edge), or with the hill falling away east of it, leaves
            // no rock to cut rooms from. The share of the ground the plans
            // use that is good rock under cover (sampled every other tile).
            let mut good = 0;
            let mut all = 0;
            for dy in (-18..=18).step_by(2) {
                for dx in (15..=59).step_by(2) {
                    all += 1;
                    let q = base.offset(dx, dy, 0);
                    if w.get(q).map_or(false, |t| t.kind == TileKind::Wall && t.mat != Mat::Bedrock && t.water == 0 && !t.aquifer) && w.surface_z(q.x, q.y) > base.z {
                        good += 1;
                    }
                }
            }
            let room = good * 200 / all;
            Some(far * 20 + (p.z - w.water_level).clamp(-5, 6) * 6 - dist + room)
        };
        // The stair goes down as far as it may (above a heavy aquifer), or
        // a level or three further when that finds the gatehouse its rock
        // (a little dearer for each level more): the best depth, and its score.
        let score = |g: &Game, p: Pos| -> Option<(i32, i32)> {
            let most = reach(g, p);
            if most < 1 {
                return None;
            }
            let deeper = if g.world.aquifer < 2 { 3 } else { 0 };
            (most..=most + deeper).filter_map(|k| score_at(g, p, k).map(|s| (s - (k - most) * 8, k))).max()
        };
        // (Out to 45 tiles: a wagon near the east edge has its halls' rock
        // to the west of it.)
        let s = (1..46)
            .flat_map(|r| (-r..=r).flat_map(move |k| [e.offset(k, -r, 0), e.offset(k, r, 0), e.offset(-r, k, 0), e.offset(r, k, 0)]))
            .filter(|p| self.world.in_bounds(Pos::new(p.x, p.y, 0)))
            .map(|p| Pos::new(p.x, p.y, self.world.surface_z(p.x, p.y)))
            .filter(|&p| open(self, p))
            .filter(|p| avoid.map_or(true, |a| (p.x - a.x).abs().max((p.y - a.y).abs()) >= 8))
            .filter_map(|p| score(self, p).map(|s| (s, p)))
            .max_by_key(|&((s, _), p)| (s, -(p.x - e.x).abs() - (p.y - e.y).abs()))
            .map(|((_, k), p)| (p, k));
        if avoid.is_some() {
            return s;
        }
        s.or_else(|| (1..12).flat_map(|i| [e.offset(i, 0, 0), e.offset(0, i, 0), e.offset(-i, 0, 0)]).find(|&p| open(self, p)).map(|p| (p, reach(self, p).max(1))))
    }

    /// Whether a dwarf could walk from the foot of the stair into the room.
    fn room_reachable(&self, o: &Overseer, r: &Room) -> bool {
        let tiles: Vec<Pos> = if r.tiles.is_empty() { box_iter(r.a, r.b).collect() } else { r.tiles.clone() };
        let Some(&inside) = tiles.iter().find(|&&p| self.world.walkable(p)) else { return false };
        let base = o.base.unwrap();
        path::astar(&self.world, base, inside, 40_000, Mover::Dwarf).is_some()
    }

    /// How much of a room has been cut out so far.
    fn room_dug(&self, r: &Room) -> f64 {
        if r.tiles.is_empty() {
            return self.dug_share(r.a, r.b);
        }
        let dug = r.tiles.iter().filter(|p| self.world.get(**p).map_or(false, |t| !t.kind.solid())).count();
        dug as f64 / r.tiles.len() as f64
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
        // The dead want burying: a tomb, and coffins in it.
        if self.tick % DAY < 150 {
            self.ov_burials(o);
        }
        // An ambition's rooms first, then the style's plan.
        let next = o.wants.first().copied().map(|k| (k, true)).or_else(|| plan.get(o.planned).map(|&k| (k, false)));
        // Past the first dormitory, a planned fortress sleeps in bedrooms.
        let next = next.map(|(k, w)| {
            if k == RoomUse::Dormitory && super::architect::sleeps_in_rooms(o) {
                (RoomUse::Bedrooms, w)
            } else {
                (k, w)
            }
        });
        // (Once nothing has fit, it only looks again daily: it's a long search.)
        let retry = o.stuck_since == 0 || self.tick % DAY < 150;
        // (Bedrooms don't count against the plan: a fort of rooms of their own has many.)
        let rooms = o.rooms.iter().filter(|r| r.kind != RoomUse::Bedrooms).count();
        if let Some((kind, wanted)) = next.filter(|_| (rooms < want || !o.wants.is_empty()) && digging < 2 && dig_jobs < 120 && retry) {
            match self.plan_room(o, kind) {
                Some(room) => {
                    let what = match (room.kind, room.shape) {
                        (RoomUse::Bedrooms, _) if room.beds.len() == 1 => "Dug out a bedroom of its own off the halls.".to_string(),
                        (RoomUse::Bedrooms, _) => format!("Dug out a row of {} bedrooms.", room.beds.len()),
                        (RoomUse::Grand, Some(s)) => format!("Began its grand work: a great {}.", s.describe()),
                        (_, Some(s)) if !matches!(s, Shape::Rect { .. }) => format!("Dug out a {} for a {:?}.", s.describe(), kind).to_lowercase().replace("dug out", "Dug out"),
                        _ => format!("Dug out a room for a {:?}.", kind).to_lowercase().replace("dug out", "Dug out"),
                    };
                    o.blocked.retain(|b| b.0 != room.kind);
                    o.stuck_since = 0;
                    o.rooms.push(room);
                    o.slots = (o.rooms.len() as i32 + 1) / 2;
                    if wanted {
                        o.wants.remove(0);
                    } else {
                        o.planned += 1;
                    }
                    Self::note(o, tick, what);
                }
                None => {
                    // Nowhere for it. A want is put by for the season (not
                    // asked for again daily); the plan itself stuck is the
                    // watchdog's business.
                    if !wanted && o.stuck_since == 0 {
                        o.stuck_since = tick;
                    }
                    if wanted {
                        let k = o.wants.remove(0);
                        o.blocked.retain(|b| b.0 != k);
                        o.blocked.push((k, tick));
                        Self::note(o, tick, format!("Found no rock to cut a {} from, for now.", room_name(k)));
                    }
                }
            }
        }
        // Fit rooms out once they're dug and somebody can walk in (checked
        // once a day: it's a long search when they can't).
        let daily = self.tick % DAY < 150;
        for i in 0..o.rooms.len() {
            if o.rooms[i].stage != 0 {
                continue;
            }
            let dug = self.room_dug(&o.rooms[i]) >= 0.9;
            if dug && daily && self.room_reachable(o, &o.rooms[i]) {
                let r = o.rooms[i].clone();
                self.ov_fit(o, &r);
                o.rooms[i].stage = 1;
            } else if daily && self.tick > o.rooms[i].since + DAY * if self.dwarves.iter().any(|d| matches!(d.task, Task::Dig { .. })) { 40 } else { 15 } && !self.room_reachable(o, &o.rooms[i]) {
                // Still out of reach after a fortnight (longer while the
                // miners are at work: a fort short of picks digs slowly): give it up.
                let r = o.rooms[i].clone();
                let tiles = if r.tiles.is_empty() { box_iter(r.a, r.b).collect() } else { r.tiles.clone() };
                for p in tiles {
                    self.designate(p, p, Desig::None);
                }
                // (And no new tunnel heads for its door or middle: they'll
                // never be dug, and a room tunnelled to them never reached.)
                o.anchors.retain(|a| !(a.z == r.a.z && a.x >= r.a.x - 1 && a.x <= r.b.x + 1 && a.y >= r.a.y - 1 && a.y <= r.b.y + 1));
                o.rooms[i].stage = 2;
                Self::note(o, tick, format!("Gave up on a {:?} nobody could reach.", r.kind).to_lowercase().replace("gave", "Gave"));
            }
        }
        // Keep a bed for everyone in the dormitories.
        if self.tick % DAY < 150 {
            let beds = self.buildings.values().filter(|b| b.kind == BuildingKind::Bed).count();
            // A planned fortress digs another row of bedrooms.
            if beds < self.alive_count() + 1
                && super::architect::sleeps_in_rooms(o)
                && !o.wants.contains(&RoomUse::Bedrooms)
                && !o.rooms.iter().any(|r| r.stage == 0 && matches!(r.kind, RoomUse::Bedrooms | RoomUse::Dormitory))
            {
                o.wants.push(RoomUse::Bedrooms);
            }
            if beds < self.alive_count() + 1 {
                for r in o.rooms.iter().filter(|r| r.kind == RoomUse::Dormitory && r.stage == 1).cloned().collect::<Vec<_>>() {
                    self.ov_grid(&r, BuildingKind::Bed, 2, 2, (self.alive_count() + 1).saturating_sub(beds));
                }
            }
        }
    }

    /// Every dwarf lying unburied wants a coffin: put more in the tomb, or
    /// dig a tomb if there is none.
    fn ov_burials(&mut self, o: &mut Overseer) {
        let unburied = self.items.values().filter(|it| it.free() && self.burial_corpse(it)).count();
        if unburied == 0 {
            return;
        }
        let free = self.buildings.values().filter(|b| b.kind == BuildingKind::Coffin && b.occupant.is_none()).count();
        if free >= unburied {
            return;
        }
        if let Some(t) = o.rooms.iter().find(|r| r.kind == RoomUse::Tomb && r.stage == 1).cloned() {
            self.ov_grid(&t, BuildingKind::Coffin, 2, 2, unburied - free);
        } else if o.blocked_lately(RoomUse::Tomb, self.tick) {
            // No rock for a tomb: coffins set out by the wagon will do.
            self.ov_coffins_outside(o, unburied - free);
        } else if !o.rooms.iter().any(|r| r.kind == RoomUse::Tomb && r.stage == 0) && !o.wants.contains(&RoomUse::Tomb) {
            o.wants.insert(0, RoomUse::Tomb);
            Self::note(o, self.tick, "The dead lie unburied. Ordered a tomb dug.".into());
        }
    }

    /// Coffins on open ground near the wagon, for the dead a tomb can't
    /// take (there's no rock left to cut one from).
    fn ov_coffins_outside(&mut self, o: &mut Overseer, n: usize) {
        let e = self.embark;
        let mut placed = 0;
        for _ in 0..n.min(8) {
            let Some(p) = self.ov_spot(BuildingKind::Coffin, e.offset(-6, -12, 0), 14) else { break };
            if self.place_building(BuildingKind::Coffin, p) {
                placed += 1;
            }
        }
        if placed > 0 {
            Self::note(o, self.tick, "Found no rock for a tomb: had coffins set out by the wagon for the dead.".into());
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
        // Rooms off a hall get their doors.
        for &d in &r.doors {
            self.place_building(BuildingKind::Door, d);
        }
        let z = match r.kind {
            RoomUse::Bedrooms => {
                for &p in &r.beds {
                    self.place_building(BuildingKind::Bed, p);
                }
                None
            }
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
                // Its last row keeps the thread, cloth and splints to hand.
                self.add_zone(Pos::new(r.a.x, r.b.y, r.a.z), r.b, ZoneKind::Stockpile(StockFilter::Medical));
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
            RoomUse::Suite | RoomUse::Library | RoomUse::Parlour | RoomUse::Industry => self.fit_interior(o, r),
            RoomUse::Grand => match r.shape {
                // A great hall to feast in, among the pillars.
                Some(Shape::Pillared { .. }) => {
                    self.ov_grid(r, BuildingKind::Table, 3, 2, 20);
                    let tables: Vec<Pos> = self.buildings.values().filter(|b| b.kind == BuildingKind::Table && r.tiles.contains(&b.pos)).map(|b| b.pos).collect();
                    for t in tables {
                        self.place_building(BuildingKind::Chair, t.offset(1, 0, 0));
                    }
                    zone(ZoneKind::Dining)
                }
                // A rotunda with the monument at its heart.
                Some(Shape::Round { r: rad }) => {
                    if !o.did("monument") && self.place_building(BuildingKind::Monument, r.a.offset(rad - 1, rad - 1, 0)) {
                        o.done.push("monument".into());
                    }
                    zone(ZoneKind::Meeting)
                }
                // A cross: a temple.
                Some(Shape::Cross { .. }) => zone(ZoneKind::Temple),
                // The long gallery: every wall carved.
                _ => {
                    self.designate(r.a.offset(-1, -1, 0), r.b.offset(1, 1, 0), Desig::Smooth);
                    zone(ZoneKind::Meeting)
                }
            },
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
            // Thread and cloth to dress wounds, splints to set bones.
            (Recipe::Splint, 3),
            (Recipe::Crutch, 1),
            (Recipe::SpinThread, 6),
            (Recipe::Weave, if pop >= 10 { 5 } else { 3 }),
            (Recipe::ClothShirt, 2),
            (Recipe::ClothTrousers, 2),
            (Recipe::Mechanism, if o.style == Style::Reckless { 0 } else { 5 }),
            (Recipe::WoodCage, if o.style == Style::Reckless { 0 } else { 4 }),
            (Recipe::Charcoal, 6),
            (Recipe::SmeltOre, 6),
        ];
        // Coffins for the dead, and a couple in hand.
        let unburied = self.items.values().filter(|it| it.free() && self.burial_corpse(it)).count() as u32;
        orders.push((Recipe::WoodCoffin, 2 + unburied));
        // With no wood to spare, the mason cuts them from stone: a tomb
        // without coffins buries nobody.
        if unburied > 0 && self.stock_of(ItemKind::Log) < 4 {
            orders.push((Recipe::StoneCoffin, 2 + unburied));
        }
        orders.push((Recipe::WoodDoor, 2 + o.rooms.iter().filter(|r| r.stage == 0).map(|r| r.doors.len() as u32).sum::<u32>()));
        // Wooden crossbows until there's a forge; then a few for practice.
        let forge = self.buildings.values().any(|b| b.built && matches!(b.kind, BuildingKind::Forge | BuildingKind::MagmaForge));
        if o.style != Style::Reckless {
            orders.push((Recipe::WoodCrossbow, if forge { 1 } else { 3 }));
            orders.push((Recipe::Bolts, if forge { 2 } else { 6 }));
        } else if self.knob("reckless.arms", 1.0) > 0.0 && !self.squads.is_empty() {
            // Its few soldiers get crossbows, and that is all the thought it gives war.
            orders.push((Recipe::WoodCrossbow, 2));
            orders.push((Recipe::Bolts, 3));
        }
        if forge {
            orders.push((Recipe::ForgeAxe, 2));
            orders.push((Recipe::ForgeHelm, 2));
            orders.push((Recipe::ForgeBreastplate, 1));
            orders.push((Recipe::ForgeShield, 1));
            orders.push(self.metal_arms_order(Recipe::ForgeBolts, 6));
            orders.push(self.metal_arms_order(Recipe::ForgeCrossbow, if o.style == Style::Warlord { 4 } else { 2 }));
        }
        self.ambition_orders(o, &mut orders);
        self.interior_orders(o, &mut orders);
        // What it raised to meet a shortage, on top.
        for &(r, keep, _) in &o.boosts {
            match orders.iter_mut().find(|x| x.0 == r) {
                Some(x) => x.1 = x.1.max(keep),
                None => orders.push((r, keep)),
            }
        }
        // Food short: no edible plant goes to the still while there's drink
        // for a week (the stills once brewed a fort's last stoutcaps).
        if self.food_short() && self.stock_of(ItemKind::Drink) as u32 >= pop * 2 {
            for x in orders.iter_mut().filter(|x| x.0 == Recipe::Brew) {
                x.1 = x.1.min(pop * 2);
            }
        }
        // Stone short and the gate still wanting its mechanisms: none of it
        // goes on blocks or crafts until they're made (a fort cut in soil
        // once turned all the stone it quarried for them into blocks).
        if self.gate_wants_stone(o) {
            let held = [Recipe::Blocks, Recipe::StoneCraft];
            for x in orders.iter_mut().filter(|x| held.contains(&x.0)) {
                x.1 = 0;
            }
            for w in self.work_orders.iter_mut().filter(|w| held.contains(&w.recipe)) {
                w.keep = 0;
            }
            for b in self.buildings.values_mut() {
                let busy = b.worker.is_some() as usize;
                let mut i = 0;
                b.orders.retain(|x| {
                    i += 1;
                    i <= busy || !held.contains(&x.recipe)
                });
            }
        }
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
        let share = if o.has(super::personality::Trait::Cautious) { share + 0.08 } else { share };
        let adults = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        let mut want = if adults >= from { ((adults as f64 * share).ceil() as usize).max(3) } else { 0 };
        if o.ambition == Some(Ambition::Muster) {
            want = want.max(self.muster_target(o));
        }
        // The reckless keep a few under arms once there are hands enough
        // (no gate, no traps: just someone to meet what comes), and take up
        // arms anyway once they've been attacked.
        if o.style == Style::Reckless {
            let few = self.knob("reckless.squad", 3.0) as usize;
            if adults >= self.knob("reckless.squad_from", 8.0) as usize {
                want = few;
            }
            if self.siege_size > 0 {
                // Once attacked, a share of the adults (as a guess at what
                // the next army will be).
                let share = (adults as f64 * self.knob("reckless.share", 0.2)).ceil() as usize;
                want = want.max(2).max(share);
            }
        }
        if want == 0 {
            return;
        }
        if self.squads.is_empty() {
            self.new_squad();
            let name = self.squads[0].name.clone();
            Self::note(o, self.tick, format!("Raised a squad, {}.", name));
        }
        // Crossbows from the carpenter arm a squad soonest; once there are
        // axes, spears and hammers enough for the squad, it fights with those.
        use super::military::UniformWeapon;
        let bows = self.stock_of(ItemKind::Crossbow) + self.dwarves.iter().filter(|d| d.alive && d.soldier && self.holds_crossbow(d)).count();
        let melee = self.items.values().filter(|it| UniformWeapon::Best.accepts(it.kind) && (it.carried_by.is_none() || self.dwarves.iter().any(|d| d.soldier && d.held == Some(it.id)))).count();
        for s in 0..self.squads.len() {
            let n = self.squads[s].members.len().max(2);
            let archers = self.archer_uniform(o, s);
            self.squads[s].weapon = archers.unwrap_or(if melee >= n || bows < 2 { UniformWeapon::Best } else { UniformWeapon::Crossbow });
        }
        let _ = o.style;
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
        // (An army the scouts saw coming counts once it's near.)
        let coming = self.incoming.as_ref().map_or(false, |i| self.tick + self.wargame_lead() >= i.0);
        // (Nor one walled up near it, in a cavern pocket or by a sunken
        // pool, with no way to the stair outside or to the halls: sheltering
        // from that would last until the fort died of thirst.)
        let danger = coming
            || self.creatures.values().any(|c| {
                c.alive
                    && c.hostile()
                    && !c.sneaking
                    // (A beaten besieger that can find no way off the map,
                    // shut out by the raised bridge, is danger only if it can
                    // get in: else the alert would hold the bridge up, and it
                    // there, until the fort died of thirst.)
                    && ((c.siege && !c.legendary() && !(c.leaving && c.path_fails >= 2))
                        || ((c.siege || c.legendary()) && near_fort(c.pos) && (self.can_reach(c, top, 20000) || self.can_reach(c, base, 20000))))
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
                    c.alive && c.siege && !c.legendary() && c.hostile() && (!near || ((c.pos.x - top.x).abs() <= 40 && (c.pos.y - top.y).abs() <= 40))
                })
            };
            let (any, at_gate) = (army(false) || coming, army(true));
            // How long the army has been on the map: a little while for those
            // outside to run in, then the bridge goes up regardless.
            if any && o.army_since == 0 {
                o.army_since = self.tick;
            } else if !any {
                o.army_since = 0;
            }
            let waited = any && self.tick > o.army_since + self.bar_patience(o);
            let long = self.tick > o.raised_at + DAY * 4;
            let want = any && (at_gate || inside || waited) && !(long && !at_gate && o.raised_at > 0);
            let bridges = self.gate_bridges(base);
            let raised = bridges.iter().any(|id| self.buildings.get(id).map_or(false, |b| b.raised));
            if let Some(lever) = self.gate_lever(base) {
                // Anyone standing by the lever (the squad holds station there) throws it at once.
                let lp = self.buildings[&lever].pos;
                let by = self.dwarves.iter().any(|d| d.alive && d.pos.z == lp.z && (d.pos.x - lp.x).abs() <= 1 && (d.pos.y - lp.y).abs() <= 1);
                if raised != want && by {
                    self.pull_lever(lever);
                } else if raised != want && !self.buildings[&lever].pull {
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
            // (A squad marching on a den holds its station there: hunts.rs.)
            let marching = self.hunts.on.as_ref().filter(|h| h.lair.is_some() && h.target == 0).map(|h| h.squad);
            for s in 0..self.squads.len() {
                let holding = self.squads[s].order == super::SquadOrder::Station;
                if !danger && marching == Some(self.squads[s].id) {
                    continue;
                }
                if danger && !charge && !holding {
                    // Marksdwarves hold back from the gate with a clear shot at it (archery.rs).
                    let st = self.archer_station(s, base).unwrap_or(base.offset(4 + s as i32, 0, 0));
                    self.order_station(s, st);
                } else if !danger && holding {
                    self.order_train(s);
                }
            }
            let _ = holding;
        }
        // Whatever the items add (overseer_hooks.rs).
        self.ov_alert_hooks(o);
    }

    /// How long an army may be on the map before the drawbridge goes up
    /// regardless, in ticks. (A cautious overseer bars the gate sooner, a
    /// bold one later, and one a siege has taught sooner still.)
    pub(super) fn bar_patience(&self, o: &Overseer) -> u64 {
        let patience = if o.has(super::personality::Trait::Cautious) {
            60
        } else if o.has(super::personality::Trait::Bold) {
            260
        } else {
            150
        };
        self.bar_sooner(o, patience)
    }

    /// What building stands on `p` (planned or built), if any.
    fn building_at(&self, p: Pos) -> Option<BuildingKind> {
        match self.world.get(p).map_or(0, |t| t.building) {
            0 => None,
            id => self.buildings.get(&id).map(|b| b.kind),
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
        self.ov_quarry(o);
        let base = o.base.unwrap();
        // The gate: a door on each row of the two-wide corridor. It isn't a
        // gate until both are there, and a door broken down is hung again.
        if self.dug_share(base.offset(1, 0, 0), base.offset(6, 1, 0)) >= 1.0 {
            let mut hung = 0;
            for dy in 0..2 {
                let p = base.offset(1, dy, 0);
                if self.building_at(p) == Some(BuildingKind::Door) || self.place_building(BuildingKind::Door, p) {
                    hung += 1;
                }
            }
            if hung == 2 && !o.did("gate") {
                o.done.push("gate".into());
                Self::note(o, self.tick, "Hung a gate at the foot of the stair, to bar when enemies come.".into());
            }
        }
        // Where the miners broke into a cavern, a door: beasts can't open one.
        // (Never on the gate's own tiles: a door where the bridge or lever
        // goes keeps the drawbridge from ever being built.)
        let gate_part = |p: Pos| p.z == base.z && (1..=3).contains(&(p.x - base.x)) && (0..=1).contains(&(p.y - base.y));
        for q in std::mem::take(&mut self.breaches) {
            let spots = std::iter::once(q).chain(crate::geom::DIRS4.iter().map(|&(dx, dy)| q.offset(dx, dy, 0)));
            let fit: Vec<Pos> = spots.filter(|&p| !gate_part(p) && self.world.get(p).map_or(false, |t| t.dug && t.kind == TileKind::Floor && t.building == 0)).collect();
            let hung = fit.into_iter().any(|p| self.place_building(BuildingKind::Door, p));
            if hung {
                Self::note(o, self.tick, "Hung a door where the miners broke into the caverns.".into());
            }
        }
        // A drawbridge behind the doors, on a lever: raised, nothing gets in.
        // Both halves and the lever, each put back if it's lost.
        let parts = [(BuildingKind::Bridge, base.offset(2, 0, 0)), (BuildingKind::Bridge, base.offset(2, 1, 0)), (BuildingKind::Lever, base.offset(3, 1, 0))];
        let missing = parts.iter().filter(|&&(k, p)| self.building_at(p) != Some(k)).count();
        if o.did("gate") && missing > 0 && self.stock_of(ItemKind::Mechanism) >= missing {
            let placed = parts.iter().filter(|&&(k, p)| self.building_at(p) == Some(k) || self.place_building(k, p)).count();
            if placed == parts.len() && !o.did("bridge") {
                o.done.push("bridge".into());
                Self::note(o, self.tick, "Began a drawbridge at the gate, worked by a lever inside.".into());
            }
        }
        // The lever works every half of the bridge (again, after a rebuild).
        if o.did("bridge") {
            let lever = self.gate_lever(base);
            let bridges = self.gate_bridges(base);
            let built = bridges.len() == 2 && bridges.iter().all(|id| self.buildings.get(id).map_or(false, |b| b.built));
            if let (Some(lever), true) = (lever, built) {
                let unlinked: Vec<u32> = bridges.iter().copied().filter(|id| !self.buildings[&lever].links.contains(id)).collect();
                for id in &unlinked {
                    let _ = self.link_lever(lever, *id);
                }
                if !unlinked.is_empty() && !o.did("linked") {
                    o.done.push("linked".into());
                    Self::note(o, self.tick, "Linked the lever to the drawbridge. The gate can be shut fast.".into());
                }
            }
        }
        // Traps once the bridge is begun: it needs the mechanisms more.
        if o.did("bridge") && !o.did("traps") && self.stock_of(ItemKind::Mechanism) >= 2 && self.stock_of(ItemKind::Cage) >= 2 {
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

    /// Whether the gate still wants mechanisms and stone is short: it holds
    /// the stone for them.
    pub(super) fn gate_wants_stone(&self, o: &Overseer) -> bool {
        if o.style == Style::Reckless || o.base.is_none() {
            return false;
        }
        let want = if o.did("linked") { self.gate_parts_missing(o) } else { 3 };
        let mechs = self.stock_of(ItemKind::Mechanism);
        let loose = self.items.values().filter(|it| it.kind == ItemKind::Boulder && it.mat.is_building_stone() && it.free()).count();
        mechs < want && loose < 12
    }

    /// Parts of the gate's drawbridge and lever not standing built (a half
    /// lost to a siege, say), each needing a mechanism to put back.
    pub(super) fn gate_parts_missing(&self, o: &Overseer) -> usize {
        let Some(base) = o.base else { return 0 };
        [(BuildingKind::Bridge, base.offset(2, 0, 0)), (BuildingKind::Bridge, base.offset(2, 1, 0)), (BuildingKind::Lever, base.offset(3, 1, 0))]
            .iter()
            .filter(|&&(k, p)| !self.buildings.values().any(|b| b.kind == k && b.pos == p && b.built))
            .count()
    }

    /// Whether `p` is in the gatehouse's walls (two deep on each side of the
    /// corridor, and round the foot of the stair), which nothing may be cut
    /// from: it would open a way round the doors and the bridge.
    pub(super) fn gatehouse(base: Pos, p: Pos) -> bool {
        p.z == base.z && (-2..=16).contains(&(p.x - base.x)) && (-2..=3).contains(&(p.y - base.y))
    }

    /// Plain stone for the gate's mechanisms. A fort whose halls are cut in
    /// soil, ore or coal has none to make them from, and its drawbridge never
    /// comes: until the lever is linked and every part of the gate stands
    /// (or while a lesson wants traps and none are set), keep a few boulders
    /// of it in hand, quarried where it shows beside the halls, else at the
    /// end of a tunnel cut through the soil to the nearest, or else down a
    /// stair at the first layer of stone.
    fn ov_quarry(&mut self, o: &mut Overseer) {
        if o.did("linked") && self.gate_parts_missing(o) == 0 && !self.traps_wanted(o) {
            return;
        }
        let base = o.base.unwrap();
        let loose = self.items.values().filter(|it| it.kind == ItemKind::Boulder && it.mat.is_building_stone() && it.free()).count();
        if loose + self.stock_of(ItemKind::Mechanism) >= self.knob("gate.stone", 4.0) as usize {
            return;
        }
        let stone = |t: &crate::world::Tile| t.kind == TileKind::Wall && t.mat.is_building_stone() && !t.aquifer && t.built_from.is_none();
        // Some already being cut: wait for it.
        if self.designations.iter().any(|p| (base.z - 4..=base.z).contains(&p.z) && {
            let t = self.world.t(*p);
            t.desig == Desig::Dig && stone(t)
        }) {
            return;
        }
        // Nor rock that holds back water or the sky, nor the waterworks'.
        let mut works: Vec<(Pos, Pos)> = Vec::new();
        if let Some(w) = self.waterworks.well.as_ref() {
            works.push((Pos::new(w.top.x, w.top.y, w.water_z), w.top));
            works.extend(w.stair.map(|s| (s, s)));
        }
        if let Some(c) = self.waterworks.cistern.as_ref() {
            works.push(c.chamber);
            works.extend([c.well, c.stair, c.shaft, c.drop, c.gate, c.breach].map(|p| (p, p)));
            works.extend(c.way.iter().map(|&p| (p, p)));
        }
        let by_works = |p: Pos| {
            works.iter().any(|&(a, b)| {
                p.x >= a.x.min(b.x) - 2 && p.x <= a.x.max(b.x) + 2 && p.y >= a.y.min(b.y) - 2 && p.y <= a.y.max(b.y) + 2 && p.z >= a.z.min(b.z) - 1 && p.z <= a.z.max(b.z) + 1
            })
        };
        let exposed = |p: Pos| {
            DIRS8.iter().any(|&(ex, ey)| {
                let q = p.offset(ex, ey, 0);
                self.world.get(q).map_or(true, |t| t.water > 0 || t.magma > 0) || self.world.outside(q)
            })
        };
        // Stone in sight, beside ground already dug (the main level, or the
        // quarry below it), anywhere round the halls.
        let mut seen: Vec<(i32, Pos)> = Vec::new();
        for dz in -4..=0 {
            for dy in -QUARRY_DY..=QUARRY_DY {
                for dx in -QUARRY_DX0..=QUARRY_DX1 {
                    if Self::gatehouse(base, base.offset(dx, dy, dz)) {
                        continue;
                    }
                    let p = base.offset(dx, dy, dz);
                    let Some(t) = self.world.get(p) else { continue };
                    let beside = || DIRS8.iter().any(|&(ex, ey)| self.world.get(p.offset(ex, ey, 0)).map_or(false, |q| q.dug && !q.kind.solid()));
                    if t.revealed && t.desig == Desig::None && stone(t) && !self.world.outside(p) && beside() && !exposed(p) && !by_works(p) {
                        seen.push((dx.abs() + dy.abs() - dz * 4, p));
                    }
                }
            }
        }
        if !seen.is_empty() {
            seen.sort();
            for &(_, p) in seen.iter().take(6) {
                self.designate(p, p, Desig::Dig);
            }
            self.count("gate.quarry", 1);
            if !o.did("quarry") {
                o.done.push("quarry".into());
                Self::note(o, self.tick, "Had plain stone quarried beside the halls, for the gate's mechanisms.".into());
            }
            return;
        }
        // None beside the halls (a fort cut in soil soon quarries what showed
        // there): tunnel through the soil from the halls to the nearest stone
        // on their levels.
        let cuttable = |g: &Game, p: Pos| {
            !Self::gatehouse(base, p)
                && g.world.get(p).map_or(false, |t| t.kind == TileKind::Wall && t.mat != Mat::Bedrock && !t.aquifer && t.desig == Desig::None && t.built_from.is_none())
                && !g.world.outside(p)
                && !exposed(p)
                && !by_works(p)
        };
        let mut best: Option<(i32, Vec<Pos>)> = None;
        for dz in -4..=0i32 {
            let z = base.z + dz;
            let (x0, x1) = ((base.x - QUARRY_DX0).max(0), (base.x + QUARRY_DX1).min(self.world.w - 1));
            let (y0, y1) = ((base.y - QUARRY_DY).max(0), (base.y + QUARRY_DY).min(self.world.h - 1));
            if z < 1 || x0 > x1 || y0 > y1 {
                continue;
            }
            let w = (x1 - x0 + 1) as usize;
            let idx = |p: Pos| (p.y - y0) as usize * w + (p.x - x0) as usize;
            let mut from: Vec<u32> = vec![u32::MAX; w * (y1 - y0 + 1) as usize];
            let mut queue = std::collections::VecDeque::new();
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let p = Pos::new(x, y, z);
                    let t = self.world.t(p);
                    if t.dug && !t.kind.solid() && !self.world.outside(p) {
                        from[idx(p)] = idx(p) as u32;
                        queue.push_back((p, 0i32));
                    }
                }
            }
            let mut hit = None;
            while let Some((p, d)) = queue.pop_front() {
                if d > 0 && stone(self.world.t(p)) {
                    hit = Some((p, d));
                    break;
                }
                if d >= QUARRY_TUNNEL {
                    continue;
                }
                for &(ex, ey) in crate::geom::DIRS4.iter() {
                    let q = p.offset(ex, ey, 0);
                    if q.x < x0 || q.x > x1 || q.y < y0 || q.y > y1 || from[idx(q)] != u32::MAX || !cuttable(self, q) {
                        continue;
                    }
                    from[idx(q)] = idx(p) as u32;
                    queue.push_back((q, d + 1));
                }
            }
            if let Some((p, d)) = hit {
                let cost = d - dz * 4;
                if best.as_ref().map_or(true, |b| cost < b.0) {
                    let mut path = vec![p];
                    let mut i = idx(p);
                    while from[i] as usize != i {
                        i = from[i] as usize;
                        let q = Pos::new(x0 + (i % w) as i32, y0 + (i / w) as i32, z);
                        if self.world.t(q).kind.solid() {
                            path.push(q);
                        }
                    }
                    best = Some((cost, path));
                }
            }
        }
        if let Some((_, path)) = best {
            // And a few more of the stone round its end.
            let mut more: Vec<Pos> = vec![path[0]];
            let mut i = 0;
            while i < more.len() && more.len() < 6 {
                for &(ex, ey) in crate::geom::DIRS4.iter() {
                    let q = more[i].offset(ex, ey, 0);
                    if more.len() < 6 && !more.contains(&q) && cuttable(self, q) && stone(self.world.t(q)) {
                        more.push(q);
                    }
                }
                i += 1;
            }
            for &p in path.iter().chain(more.iter()) {
                self.designate(p, p, Desig::Dig);
            }
            self.count("gate.quarry", 1);
            self.count("gate.tunnel", 1);
            if !o.did("quarry") {
                o.done.push("quarry".into());
            }
            Self::note(o, self.tick, format!("No plain stone left beside the halls: had a tunnel of {} cut to the nearest, for the gate's mechanisms.", path.len() - 1));
            return;
        }
        // None at all on those levels: sink the inner stair to the first
        // layer of stone (not into an aquifer, nor into the caverns) and cut a
        // pocket there.
        if o.did("quarry-stair") {
            return;
        }
        let roof = self.world.caverns.iter().map(|c| c.1).max().unwrap_or(0);
        let wet = |g: &Game, p: Pos| g.world.get(p).map_or(true, |t| t.aquifer || t.water > 0);
        // The inner stair's spot first; else a bare floor of the halls, the
        // nearest first (the inner stair may stand over a cavern).
        let mut spots: Vec<(i32, Pos)> = Vec::new();
        for dy in -QUARRY_DY..=QUARRY_DY {
            for dx in -QUARRY_DX0..=QUARRY_DX1 {
                let p = base.offset(dx, dy, 0);
                if Self::gatehouse(base, p) || !self.world.in_bounds(p) {
                    continue;
                }
                let t = self.world.t(p);
                if t.dug && t.kind == TileKind::Floor && t.building == 0 && t.zone == 0 && t.desig == Desig::None && !self.world.outside(p) {
                    spots.push((dx.abs() + dy.abs(), p));
                }
            }
        }
        spots.sort();
        let inner = base.offset(9, 1, 0);
        for s in std::iter::once(inner).chain(spots.into_iter().map(|s| s.1).take(300)) {
            let (sx, sy) = (s.x, s.y);
            for dz in 1..=4 {
                let z = base.z - dz;
                // (The level just over a cavern will do, where the rock under
                // the pocket is whole: the halls often sit right on one.)
                if z < 3 || z <= roof || wet(self, Pos::new(sx, sy, z)) || !self.world.solid(Pos::new(sx, sy, z)) {
                    break;
                }
                let pocket: Vec<Pos> = box_iter(Pos::new(sx - 2, sy - 1, z), Pos::new(sx + 2, sy + 1, z)).collect();
                if pocket.iter().any(|&p| wet(self, p) || !self.world.in_bounds(p) || (z == roof + 1 && !self.world.solid(p.down()))) {
                    continue;
                }
                if pocket.iter().filter(|&&p| stone(self.world.t(p))).count() >= 4 {
                    self.designate(pocket[0], *pocket.last().unwrap(), Desig::Dig);
                    self.designate(Pos::new(sx, sy, base.z), Pos::new(sx, sy, z), Desig::StairUD);
                    if !o.did("quarry") {
                        o.done.push("quarry".into());
                    }
                    o.done.push("quarry-stair".into());
                    self.count("gate.quarry", 1);
                    let what = if s == inner { "the inner stair" } else { "a stair from the halls" };
                    Self::note(o, self.tick, format!("No plain stone in the halls for the gate's mechanisms: sank {} {} levels to quarry some.", what, dz));
                    return;
                }
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
        // A doctor always, hospital or not: the wounded can't wait for one.
        need.push((Labor::Medicine, 1 + pop / 15));
        if o.style == Style::Builder {
            need.push((Labor::Engraving, 2));
        }
        // While food is short, the field hands stay in the fields (larder.rs).
        let hands = self.field_hands();
        for (labor, want) in need {
            let have = self.dwarves.iter().filter(|d| d.alive && !d.is_child() && !d.soldier && !d.insane && d.has(labor)).count();
            if have >= want {
                continue;
            }
            let mut cands: Vec<(u32, usize, u32)> = self
                .dwarves
                .iter()
                .filter(|d| d.alive && !d.is_child() && !d.soldier && !d.insane && !d.has(labor))
                .filter(|d| Self::field_hand_may(labor) || !hands.contains(&d.id))
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
        // What the announcements say the fortress is short of.
        self.ov_troubles(o);
        // Keep the wood coming: chop further out when it runs low.
        if self.stock_of(ItemKind::Log) < self.ov_wood_floor() {
            let chopping = self.designations.iter().filter(|p| self.world.t(**p).desig == Desig::Chop).count();
            if chopping < 5 {
                self.chop_further(&mut o.done);
            }
        }
        self.ov_defences(o);
        // More fields as the fortress grows.
        let farm: usize = self.zones.values().filter(|z| z.kind == ZoneKind::Farm).map(|z| z.tiles.len()).sum();
        if farm < self.alive_count() * 4 {
            self.ov_more_fields(o);
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
                let mut tiles: Vec<Pos> = box_iter(base.offset(1, 0, 0), base.offset(14, 1, 0)).collect();
                for r in &o.rooms {
                    tiles.extend(box_iter(r.a, r.b));
                }
                tiles.extend(o.anchors.iter().copied());
                // The inner stair down to every level opened.
                let deepest = Self::level_z(o, o.level);
                tiles.extend((deepest..=base.z).map(|z| Pos::new(base.x + 9, base.y + 1, z)));
                self.burrow_add(i, &tiles);
                // Every hall dug underground so far, so the sheltering can
                // still reach every room (and its barrels).
                let dug: Vec<Pos> = (0..self.world.tiles.len())
                    .filter(|&k| {
                        let t = &self.world.tiles[k];
                        t.dug && !t.kind.solid() || matches!(t.desig, Desig::Dig | Desig::StairUD)
                    })
                    .map(|k| self.world.pos_of(k))
                    // (Not the stair from the surface: that's the way in, outside the gate.)
                    .filter(|&p| !self.world.outside(p) && p.z <= base.z && (p.x, p.y) != (base.x, base.y) && (p.x - base.x, p.y - base.y) != (-1, 0))
                    .collect();
                self.burrow_add(i, &dug);
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
        // Stuck? Step in (watchdog.rs).
        self.ov_watchdog(o);
        // Whatever the items add (overseer_hooks.rs).
        self.ov_daily_hooks(o);
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
        // (What it meant to buy to break a circle of troubles, before all.)
        let rank = |k: ItemKind| match k {
            _ if o.wanted.contains(&k) => 0,
            ItemKind::Drink | ItemKind::Meal | ItemKind::Meat | ItemKind::Plant | ItemKind::Fish => 1,
            ItemKind::Bar => 2,
            ItemKind::Cloth | ItemKind::Leather | ItemKind::Thread => 3,
            _ => 4,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub verdict: String,
    /// Who oversaw it ("Urist Gemcutter, cautious and vain").
    #[serde(default)]
    pub overseer: String,
    /// The last word on it: what ended it, or its greatest moment.
    pub epitaph: String,
    /// How each dwarf died and each siege went.
    #[serde(default)]
    pub deaths: Vec<super::annals::DeathNote>,
    #[serde(default)]
    pub sieges: Vec<super::annals::SiegeNote>,
    /// Dwarves alive at the end of each year it lasted into.
    #[serde(default)]
    pub alive_by_year: Vec<usize>,
    /// What happened, in order: (day, what).
    #[serde(default)]
    pub story: Vec<(u64, String)>,
    /// The fort's counters at the end (`Game::count`).
    #[serde(default)]
    pub counters: BTreeMap<String, u32>,
    /// The knobs the batch set (`--knob`); empty for the defaults.
    #[serde(default)]
    pub knobs: BTreeMap<String, f64>,
}

/// A fresh fortress played by `style` up to the start of `until_day` (or
/// until everyone is dead), for tests that look inside a trial. It's the same
/// fort `run_trial` plays: the trial's bookkeeping doesn't touch the game.
#[cfg_attr(not(test), allow(dead_code))]
pub fn trial_game(seed: u64, style: Style, until_day: u64) -> Game {
    trial_game_with(seed, style, until_day, &Knobs::default())
}

/// `trial_game` with `knobs` changed from their defaults.
pub fn trial_game_with(seed: u64, style: Style, until_day: u64, knobs: &Knobs) -> Game {
    let mut g = Game::new(seed);
    g.knobs = knobs.clone();
    g.set_overseer(Some(style));
    while g.tick < until_day * DAY && g.alive_count() > 0 {
        g.step();
    }
    g
}

/// Tests: the first fort of the `n` seeds from `first` on that, played
/// by `style` to the start of `day`, is as `want` asks (it has grown its
/// halls, linked its gate...), and the seeds passed over. Tests that need a
/// fort in some state pick it this way rather than by name: each new way of
/// making the land changes every seed's fort, and named forts were lost.
#[cfg(test)]
pub fn fort_where(style: Style, first: u64, n: u64, day: u64, want: impl Fn(&Game) -> bool) -> (Option<(u64, Game)>, Vec<u64>) {
    let mut passed = Vec::new();
    for seed in first..first + n {
        let g = trial_game(seed, style, day);
        if g.alive_count() > 0 && want(&g) {
            return (Some((seed, g)), passed);
        }
        passed.push(seed);
    }
    (None, passed)
}

/// Play one fortress with `style` for up to `years`, from a fresh world,
/// with `knobs` changed from their defaults.
pub fn run_trial(seed: u64, style: Style, years: u64, knobs: &Knobs) -> Trial {
    let mut g = Game::new(seed);
    g.knobs = knobs.clone();
    g.set_overseer(Some(style));
    let mut last_death = String::new();
    let end = years * YEAR;
    let mut seen = 0u64;
    let mut story: Vec<(u64, String)> = Vec::new();
    let mut alive_by_year = Vec::new();
    let mut ambitions_told = 0;
    let mut prof_nodes = 0u64;
    let mut sieges_seen = 0;
    while g.tick < end && g.alive_count() > 0 {
        g.step();
        if g.tick % YEAR == 0 {
            alive_by_year.push(g.alive_count());
        }
        // TRIAL_SIEGE: what the fort has when each siege arrives.
        if g.annals.sieges.len() > sieges_seen {
            sieges_seen = g.annals.sieges.len();
            if std::env::var("TRIAL_SIEGE").is_ok() {
                let kit = |d: &Dwarf| {
                    let w = d.held.and_then(|id| g.items.get(&id)).map_or("none".to_string(), |it| format!("{:?} {:?}", it.mat, it.kind));
                    let a: Vec<String> = d.armor.iter().filter_map(|id| g.items.get(id)).map(|it| format!("{:?}", it.kind)).collect();
                    format!("{} [{}] skill {} bolts {}", w, a.join(","), d.fight_level(), d.ammo)
                };
                let soldiers: Vec<String> = g.dwarves.iter().filter(|d| d.alive && d.soldier).map(kit).collect();
                let s = g.annals.sieges.last().unwrap();
                eprintln!(
                    "SIEGE {} {} #{} day {}: {} {} | pop {} soldiers {} | weapons in stock {} armor {} | bridge {} traps {} burrow {} | soldiers: {:?}",
                    style.name(),
                    seed,
                    sieges_seen,
                    g.tick / DAY,
                    s.size,
                    s.foe,
                    g.alive_count(),
                    soldiers.len(),
                    g.items.values().filter(|it| it.kind.weapon_damage().is_some() && it.kind != ItemKind::Pick && it.carried_by.is_none()).count(),
                    g.items.values().filter(|it| it.kind.armor_slot().is_some() && it.carried_by.is_none()).count(),
                    g.buildings.values().any(|b| b.kind == BuildingKind::Bridge && b.built),
                    g.buildings.values().filter(|b| b.kind.is_trap() && b.built).count(),
                    g.alert_burrow.is_some(),
                    soldiers
                );
                let o = g.overseer.as_ref().unwrap();
                let base = o.base.unwrap();
                let parts: Vec<String> = g.buildings.values().filter(|b| (b.pos.x - base.x).abs() <= 4 && (b.pos.y - base.y).abs() <= 2 && b.pos.z == base.z).map(|b| format!("{:?}{}", b.kind, if b.built { "" } else { "(planned)" })).collect();
                eprintln!(
                    "      done {:?} | mechanisms {} | mechanic shop {} | gate parts {:?}",
                    o.done.iter().filter(|d| ["gate", "bridge", "linked", "traps", "burrow"].contains(&d.as_str())).collect::<Vec<_>>(),
                    g.stock_of(ItemKind::Mechanism),
                    g.buildings.values().any(|b| b.kind == BuildingKind::Mechanic && b.built),
                    parts
                );
            }
        }
        // The fort's story: arrivals, sieges and monsters, deaths, milestones, ambitions.
        if g.tick % 50 == 0 {
            for l in g.log.iter().filter(|l| l.tick > seen) {
                let t = &l.text;
                let tell = l.color == (255, 60, 60)
                    || t.contains("arrived")
                    || t.contains("march to war")
                    || t.contains("up from the caverns")
                    || t.contains("descends upon")
                    || t.contains("are fleeing")
                    || t.contains("bashes down")
                    || t.contains("Milestone")
                    || t.contains("raises the dead");
                // (Not the seasons, nor the merchants.)
                let routine = t.contains("Merchants") || ["Spring", "Summer", "Autumn", "Winter"].iter().any(|s| t.starts_with(s));
                if tell && !routine && story.len() < 400 {
                    story.push((l.tick / DAY, t.clone()));
                }
            }
            seen = g.tick;
            if let Some(o) = g.overseer.as_ref() {
                for (tick, a) in o.ambitions.iter().skip(ambitions_told) {
                    story.push((tick / DAY, format!("The overseer took up {}.", a.name())));
                }
                ambitions_told = o.ambitions.len();
            }
        }
        // TRIAL_PROF=1: every ten days, where the simulation's time went.
        if g.tick == 1 && std::env::var("TRIAL_PROF").is_ok() {
            g.prof = Some([0.0; 6]);
        }
        if g.tick % (DAY * 10) == 0 && g.prof.is_some() {
            use std::sync::atomic::Ordering::Relaxed;
            let p = g.prof.replace([0.0; 6]).unwrap();
            let n = crate::path::SEARCH_NODES.load(Relaxed) + crate::path::ASTAR_NODES.load(Relaxed);
            let per = (DAY * 10) as f64;
            eprintln!(
                "  day {:>4}: {:.3} ms/tick (worst {:.1})  water {:.3} overseer {:.3} dwarves {:.3} creatures {:.3} rest {:.3}  nodes/tick {:.0}  pop {} creatures {} items {}",
                g.tick / DAY,
                p[4] / per,
                p[5],
                p[0] / per,
                p[1] / per,
                p[2] / per,
                p[3] / per,
                (p[4] - p[0] - p[1] - p[2] - p[3]) / per,
                (n - prof_nodes) as f64 / per,
                g.dwarves.iter().filter(|d| d.alive).count(),
                g.creatures.values().filter(|c| c.alive).count(),
                g.items.len()
            );
            prof_nodes = n;
            let f: Vec<String> = super::finders::prof_take().iter().take(6).map(|(k, ms, n, h)| format!("{} {:.3} ({}/{})", k, ms / per, h, n)).collect();
            eprintln!("            finders ms/tick: {}", f.join(", "));
        }
        if g.tick % DAY == 0 && std::env::var("TRIAL_PACE").is_ok() {
            eprintln!("  day {} at {:?}: {} rooms, {} designations, {} creatures", g.tick / DAY, std::time::Instant::now(), g.overseer.as_ref().map_or(0, |o| o.rooms.len()), g.designations.len(), g.creatures.len());
        }
        if g.tick % DAY == 0 {
            if let Some(l) = g.log.iter().rev().find(|l| l.color == (255, 60, 60)) {
                last_death = l.text.clone();
            }
        }
        if std::env::var("TRIAL_PROBE").ok().and_then(|s| s.parse::<u64>().ok()).map_or(false, |d| g.tick == DAY * d) {
            let drinks: Vec<Pos> = g.items.values().filter(|it| it.kind == ItemKind::Drink).map(|it| it.pos).take(3).collect();
            let o = g.overseer.as_ref().unwrap();
            eprintln!("  probe: top {:?} base {:?} drinks {:?}", o.top, o.base, drinks);
            eprintln!("  alert {} civ {} burrows {}", g.alert, g.civ_alert, g.burrows.len());
            for it in g.items.values().filter(|it| it.kind == ItemKind::Drink).take(4) {
                eprintln!("    drink {} at {:?} claimed {:?} forbidden {} container {:?} depot {} dump {} carried {:?} zone {}", it.id, it.pos, it.claimed_by, it.forbidden, it.container, it.to_depot, it.dump, it.carried_by, g.world.t(it.pos).zone);
            }
            if let (Some(top), Some(base)) = (o.top, o.base) {
                for z in base.z..=top.z {
                    let t = g.world.t(Pos::new(top.x, top.y, z));
                    eprintln!("    stair z{} {:?} bld {} door {}", z, t.kind, t.building, t.door);
                }
            }
            for d in g.dwarves.iter().filter(|d| d.alive) {
                let reach = drinks.first().map(|&q| path::astar(&g.world, d.pos, q, 20000, Mover::Dwarf).is_some());
                eprintln!("    {} at {:?} task {:?} thirst {} reach {:?}", d.name, d.pos, d.task, d.thirst, reach);
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
    // Close the books on a siege the fort fell to.
    g.annals_tick();
    // Deaths in the story say what did it.
    for (_, text) in story.iter_mut() {
        if let Some(d) = g.annals.deaths.iter().find(|d| d.killer.is_some() && text.starts_with(&d.name)) {
            text.push_str(&format!(" ({})", d.killer.as_deref().unwrap_or("")));
        }
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
    }
    .to_string();
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
        overseer: o.title(),
        deaths: g.annals.deaths.clone(),
        sieges: g.annals.sieges.clone(),
        alive_by_year,
        story,
        counters: g.annals.counters.clone(),
        knobs: g.knobs.set.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reckless_keep_a_few_under_arms() {
        let soldiers = |squad: f64| {
            let mut g = Game::new(5);
            g.knobs.set.insert("reckless.squad_from".into(), 1.0);
            g.knobs.set.insert("reckless.squad".into(), squad);
            let mut o = Overseer::new(Style::Reckless);
            g.ov_army(&mut o);
            g.dwarves.iter().filter(|d| d.alive && g.squad_of(d.id).is_some()).count()
        };
        assert_eq!(soldiers(3.0), 3, "three under arms once there are hands enough");
        assert_eq!(soldiers(0.0), 0, "and none with the knob at zero");
    }

    /// A cave horror walled up in a pocket of rock beside the halls sounds
    /// no alarm (the fort would shelter from it until it died of thirst);
    /// the same horror by the stair outside does.
    #[test]
    fn a_walled_up_horror_raises_no_alarm() {
        let mut g = trial_game(5000, Style::Steady, 30);
        g.creatures.retain(|_, c| c.tame);
        let mut o = g.overseer.take().unwrap();
        let (top, base) = (o.top.unwrap(), o.base.unwrap());
        let sealed = |g: &Game, p: Pos| (-1..=1).all(|dx| (-1..=1).all(|dy| (-1..=1).all(|dz| g.world.in_bounds(p.offset(dx, dy, dz)) && g.world.solid(p.offset(dx, dy, dz)))));
        let pocket = (8..25)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| base.offset(dx, dy, -1))))
            .find(|&p| sealed(&g, p))
            .expect("solid rock near the halls");
        g.world.t_mut(pocket).kind = TileKind::Floor;
        g.dig_version += 1;
        let horror = g.make_cave_horror(pocket);
        g.ov_alert(&mut o);
        assert!(!g.alert, "a horror shut in the rock at {:?} (base {:?}) raised the alarm", pocket, base);
        // Let out by the stair outside, it does.
        let out = (-2..=2).flat_map(|dy| (-2..=2).map(move |dx| top.offset(dx, dy, 0))).find(|&p| p != top && g.world.walkable(p)).expect("ground by the stair");
        g.creatures.get_mut(&horror).unwrap().pos = out;
        g.ov_alert(&mut o);
        assert!(g.alert, "a horror at the stair raised no alarm");
    }

    /// New fields go to each of the eight spots at most once: a spot that
    /// takes no field is put aside, the fields made are counted, and the
    /// list of what's done stops growing (a save once had 408 "farm"s).
    #[test]
    fn more_fields_try_each_spot_once() {
        let mut g = trial_game(5000, Style::Steady, 2);
        let mut o = g.overseer.take().unwrap();
        // Nothing can be farmed at three of the spots: their ground is bare rock.
        let e = g.embark;
        for &(dx, dy) in &FIELD_SPOTS[1..4] {
            for p in box_iter(e.offset(dx, dy, -1), e.offset(dx + 7, dy + 5, 1)) {
                if g.world.in_bounds(p) && g.world.t(p).mat.is_soil() {
                    g.world.t_mut(p).mat = crate::material::Mat::Granite;
                }
            }
        }
        let zones = g.zones.len();
        let made = (0..40).filter(|_| g.ov_more_fields(&mut o)).count();
        let farms: Vec<&String> = o.done.iter().filter(|d| d.starts_with("farm")).collect();
        assert_eq!(farms.len(), 8, "one entry per spot: {:?}", farms);
        assert!(farms.iter().filter(|d| d.ends_with('x')).count() >= 3, "the rock spots weren't refused: {:?}", farms);
        assert_eq!(made, farms.iter().filter(|d| !d.ends_with('x')).count());
        assert_eq!(g.zones.len(), zones + made);
        // An old save's hundreds of entries are pruned, and nothing more is tried.
        for k in 8..408 {
            o.done.push(format!("farm{}", k));
        }
        assert!(!g.ov_more_fields(&mut o));
        assert_eq!(o.done.iter().filter(|d| d.starts_with("farm")).count(), 8);
        assert_eq!(g.zones.len(), zones + made);
    }

    /// Replay a batch fort through one of its sieges, a few lines a tick:
    /// SEED, STYLE, SIEGE (which one, from 1) in the environment.
    #[test]
    #[ignore]
    fn siege_replay() {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or(d.into());
        let seed: u64 = env("SEED", "5005").parse().unwrap();
        let style = Style::parse(&env("STYLE", "steady")).unwrap();
        let nth: usize = env("SIEGE", "2").parse().unwrap();
        let mut g = Game::new(seed);
        g.set_overseer(Some(style));
        while g.annals.sieges.len() < nth && g.alive_count() > 0 && g.tick < 3 * YEAR {
            g.step();
        }
        let start = g.tick;
        let o = g.overseer.clone().unwrap();
        let (top, base) = (o.top.unwrap(), o.base.unwrap());
        eprintln!("siege {} at day {}: top {:?} base {:?} pop {}", nth, start / DAY, top, base, g.alive_count());
        let doors: Vec<(Pos, bool, bool)> = g.buildings.values().filter(|b| b.kind == BuildingKind::Door && (b.pos.x - base.x).abs() <= 3 && (b.pos.y - base.y).abs() <= 3).map(|b| (b.pos, b.built, b.locked)).collect();
        eprintln!("gate doors near the base: {:?}; done {:?}", doors, o.done);
        for y in base.y - 2..=base.y + 3 {
            let row: String = (base.x - 2..base.x + 8).map(|x| {
                let t = g.world.t(Pos::new(x, y, base.z));
                if (x, y) == (base.x, base.y) { 'X' } else if t.building != 0 { 'b' } else if t.kind.solid() { '#' } else { '.' }
            }).collect();
            eprintln!("    {}", row);
        }
        let shelter = g.burrows.iter().find(|b| Some(b.bit) == g.alert_burrow).map(|b| b.mask());
        let mut seen = g.log.len();
        while g.tick < start + DAY && g.alive_count() > 0 {
            g.step();
            if g.tick % 20 == 0 {
                let in_shelter = |p: Pos| shelter.map_or(false, |m| g.world.get(p).map_or(false, |t| t.burrows & m != 0));
                let civ: Vec<&Dwarf> = g.dwarves.iter().filter(|d| d.alive && !d.soldier).collect();
                let out = civ.iter().filter(|d| !in_shelter(d.pos)).count();
                let outside = civ.iter().filter(|d| g.world.outside(d.pos)).count();
                let gobs: Vec<Pos> = g.creatures.values().filter(|c| c.alive && c.siege && !c.legendary()).map(|c| c.pos).collect();
                let near_gate = gobs.iter().filter(|p| (p.x - top.x).abs() <= 12 && (p.y - top.y).abs() <= 12).count();
                let inside = gobs.iter().filter(|p| in_shelter(**p)).count();
                let bridge = g.buildings.values().filter(|b| matches!(b.kind, BuildingKind::Bridge | BuildingKind::Lever) && (b.pos.x - base.x).abs() <= 4 && (b.pos.y - base.y).abs() <= 2).map(|b| format!("{:?} raised {} pull {}", b.kind, b.raised, b.pull)).collect::<Vec<_>>();
                let nearest = gobs.iter().map(|p| (p.x - top.x).abs().max((p.y - top.y).abs())).min().unwrap_or(-1);
                let _ = nearest;
                eprintln!(
                    "t+{:>4} alert {} civ {} {:?} | civilians {} (out of shelter {}, under sky {}) | foes {} near gate {} in shelter {}",
                    g.tick - start, g.alert as u8, g.civ_alert as u8, bridge, civ.len(), out, outside, gobs.len(), near_gate, inside
                );
                for l in &g.log[seen.min(g.log.len())..] {
                    if l.text.contains(" in the ") && !l.text.contains("deflects") || l.color == (255, 60, 60) {
                        // Where the struck dwarf stands.
                        let who = g.dwarves.iter().find(|d| l.text.contains(&format!(" {} in the", d.name)) || l.text.starts_with(&d.name));
                        let wh = who.map_or(String::new(), |d| format!(" [{}{} at {:?}]", if d.soldier { "soldier" } else { "civilian" }, if in_shelter(d.pos) { ", sheltered" } else { "" }, d.pos));
                        eprintln!("        {}{}", l.text, wh);
                    }
                }
                seen = g.log.len();
            }
        }
    }

    #[test]
    fn a_door_goes_where_the_caverns_were_broken_into() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        for _ in 0..DAY * 4 {
            g.step();
        }
        let base = g.overseer.as_ref().unwrap().base.unwrap();
        // A spot in the dug corridor, as if a cavern opened beside it.
        let q = (5..14).map(|i| base.offset(i, 0, 0)).find(|&p| g.world.t(p).dug && g.world.t(p).kind == TileKind::Floor && g.world.t(p).building == 0).expect("a dug corridor");
        g.breaches.push(q);
        for _ in 0..DAY {
            g.step();
        }
        // (The miners may have broken in somewhere else meanwhile.)
        assert!(!g.breaches.contains(&q), "the breach was never seen to");
        let near = |p: Pos| (p.x - q.x).abs() + (p.y - q.y).abs() <= 1 && p.z == q.z;
        assert!(g.buildings.values().any(|b| b.kind == BuildingKind::Door && near(b.pos)), "no door at the breach");
    }

    #[test]
    fn the_shelter_reaches_every_room() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        let mut days = 0;
        while days < 80 && !(g.overseer.as_ref().unwrap().did("burrow") && g.overseer.as_ref().unwrap().rooms.iter().filter(|r| r.stage == 1).count() >= 5) {
            for _ in 0..DAY {
                g.step();
            }
            days += 1;
        }
        let o = g.overseer.clone().unwrap();
        assert!(o.did("burrow"), "no shelter after {} days", days);
        let mask = g.burrows.iter().find(|b| Some(b.bit) == g.alert_burrow).unwrap().mask();
        let inside = |p: Pos| g.world.get(p).map_or(false, |t| t.burrows & mask != 0);
        // Walk the shelter from the foot of the stair.
        let base = o.base.unwrap();
        let mut seen = std::collections::HashSet::new();
        let mut todo = vec![base.offset(4, 0, 0)];
        let mut nb = Vec::new();
        while let Some(p) = todo.pop() {
            if !seen.insert(p) {
                continue;
            }
            g.world.neighbors(p, &mut nb, Mover::Dwarf);
            for &(q, _) in nb.iter() {
                if inside(q) && !seen.contains(&q) {
                    todo.push(q);
                }
            }
        }
        for r in o.rooms.iter().filter(|r| r.stage == 1) {
            let tiles = if r.tiles.is_empty() { box_iter(r.a, r.b).collect() } else { r.tiles.clone() };
            if !tiles.iter().any(|p| seen.contains(p)) {
                eprintln!("base {:?}; room tiles in shelter {}/{}", base, tiles.iter().filter(|p| inside(**p)).count(), tiles.len());
                if let Some(path) = path::astar(&g.world, base, tiles[tiles.len() / 2], 20000, Mover::Dwarf) {
                    let out: Vec<(Pos, bool)> = path.iter().copied().filter(|p| !inside(*p)).take(8).map(|p| (p, g.world.outside(p))).collect();
                    eprintln!("  path tiles outside the shelter: {:?}", out);
                }
            }
            assert!(tiles.iter().any(|p| seen.contains(p)), "the shelter doesn't reach the {:?} at {:?} (day {})", r.kind, r.a, days);
        }
    }

    #[test]
    fn the_gate_is_whole_and_mended() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        let mut days = 0;
        while !g.overseer.as_ref().unwrap().did("gate") && days < 30 {
            for _ in 0..DAY {
                g.step();
            }
            days += 1;
        }
        let base = g.overseer.as_ref().unwrap().base.unwrap();
        let door_at = |g: &Game, p: Pos| g.buildings.values().find(|b| b.kind == BuildingKind::Door && b.pos == p).map(|b| b.id);
        let (a, b) = (door_at(&g, base.offset(1, 0, 0)), door_at(&g, base.offset(1, 1, 0)));
        assert!(a.is_some() && b.is_some(), "the gate needs a door on both rows of the corridor (after {} days)", days);
        // A door broken down is hung again.
        g.remove_building(b.unwrap());
        for _ in 0..DAY {
            g.step();
        }
        assert!(door_at(&g, base.offset(1, 1, 0)).is_some(), "the broken door wasn't replaced");
    }

    #[test]
    #[ignore]
    fn where_the_flood_comes_from() {
        let mut g = Game::new(5023);
        g.set_overseer(Some(Style::Warlord));
        let mut first = None;
        while g.tick < 30 * DAY {
            g.step();
            if let Some(base) = g.overseer.as_ref().and_then(|o| o.base) {
                let wet = (base.x..base.x + 30).any(|x| (base.y - 2..base.y + 4).any(|y| g.world.t(Pos::new(x, y, base.z)).water > 0));
                if wet && first.is_none() {
                    first = Some(g.tick);
                    break;
                }
            }
        }
        let o = g.overseer.clone().unwrap();
        let base = o.base.unwrap();
        eprintln!("first water in the halls at day {:?}; base {:?} water level {}", first.map(|t| t / DAY), base, g.world.water_level);
        let wet: Vec<(Pos, u8, bool)> = (base.x..base.x + 30).flat_map(|x| (base.y - 2..base.y + 4).map(move |y| Pos::new(x, y, base.z))).filter(|p| g.world.t(*p).water > 0).map(|p| (p, g.world.t(p).water, g.world.outside(p))).take(10).collect();
        eprintln!("wet tiles: {:?} surface there {:?}", wet, wet.first().map(|w| g.world.surface_z(w.0.x, w.0.y)));
        for z in [base.z + 2, base.z + 1, base.z] {
            eprintln!(" z{}", z);
            for y in base.y - 8..base.y + 9 {
                let row: String = (base.x - 4..base.x + 50)
                    .map(|x| {
                        let t = g.world.t(Pos::new(x, y, z));
                        if (x, y) == (base.x, base.y) { 'X' } else if t.water > 0 { char::from_digit(t.water as u32, 10).unwrap() } else if t.desig != Desig::None { '+' } else if t.kind.solid() { '#' } else { '.' }
                    })
                    .collect();
                eprintln!("   {}", row);
            }
        }
    }

    #[test]
    #[ignore]
    fn watch_a_thirsty_dwarf() {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or(d.into());
        let seed: u64 = env("SEED", "5012").parse().unwrap();
        let style = Style::parse(&env("STYLE", "delver")).unwrap();
        let mut g = Game::new(seed);
        g.set_overseer(Some(style));
        let limit: u32 = env("THIRST", "11000").parse().unwrap();
        let hungry = env("WHAT", "thirst") == "hunger";
        let over = |d: &Dwarf| d.alive && if hungry { d.hunger > limit } else { d.thirst > limit };
        while g.tick < 3 * YEAR && g.alive_count() > 0 && !g.dwarves.iter().any(|d| over(d)) {
            g.step();
        }
        let Some(i) = g.dwarves.iter().position(|d| over(d)) else { eprintln!("nobody got that thirsty"); return };
        {
            let d = &g.dwarves[i];
            for it in g.items.values().filter(|it| it.is_food()).take(8) {
                let reach = path::astar(&g.world, d.pos, it.pos, 40_000, Mover::Dwarf).map(|p| p.len());
                eprintln!("  food {:?} {:?} at {:?} claimed {:?} forbidden {} carried {:?} container {:?} reach {:?}", it.kind, it.mat, it.pos, it.claimed_by, it.forbidden, it.carried_by, it.container, reach);
            }
            let o = g.overseer.as_ref().unwrap();
            let r = |q: Pos| path::astar(&g.world, d.pos, q, 60_000, Mover::Dwarf).map(|p| p.len());
            eprintln!("  wagon {:?} reach {:?} | top {:?} reach {:?} | base {:?} reach {:?} | outside {} | surface here {}", g.embark, r(g.embark), o.top, o.top.and_then(r), o.base, o.base.and_then(r), g.world.outside(d.pos), g.world.surface_z(d.pos.x, d.pos.y));
            let mut nb = Vec::new();
            g.world.neighbors(d.pos, &mut nb, Mover::Dwarf);
            let reach_n = path::search(&g.world, d.pos, 100_000, Mover::Dwarf, |_| false);
            let _ = reach_n;
            let mut seen = std::collections::HashSet::new();
            let mut todo = vec![d.pos];
            while let Some(p) = todo.pop() { if seen.len() > 20000 || !seen.insert(p) { continue; } g.world.neighbors(p, &mut nb, Mover::Dwarf); for &(q, _) in nb.iter() { if !seen.contains(&q) { todo.push(q); } } }
            let zs: std::collections::BTreeSet<i32> = seen.iter().map(|p| p.z).collect();
            for y in 25..30 {
                let row: Vec<String> = (36..60).map(|x| {
                    let t = g.world.t(Pos::new(x, y, 15));
                    let b = g.buildings.get(&t.building).map_or(String::new(), |b| format!("{:?}{}", b.kind, if b.locked { "L" } else { "" }).chars().take(3).collect());
                    if t.kind.solid() { "#".to_string() } else if t.water > 0 { format!("~{}", t.water) } else if !b.is_empty() { b } else if t.door != 0 { format!("d{}", t.door) } else { format!("{:?}", t.kind).chars().take(1).collect() }
                }).collect();
                eprintln!("  y{} {}", y, row.join(" "));
            }
            let top = o.top.unwrap();
            for z in (top.z - 4)..=(top.z + 1) {
                let tt = g.world.t(Pos::new(top.x, top.y, z));
                let around: Vec<i32> = (-3..=3).map(|dx| g.world.t(Pos::new(top.x + dx, top.y, z)).water as i32).collect();
                eprintln!("    stair z{} {:?} water {} row water {:?}", z, tt.kind, tt.water, around);
            }
            eprintln!("    water level {} | surface at top {}", g.world.water_level, g.world.surface_z(top.x, top.y));
            eprintln!("  his region: {} tiles, levels {:?}, x {}..{}, y {}..{}", seen.len(), zs, seen.iter().map(|p| p.x).min().unwrap(), seen.iter().map(|p| p.x).max().unwrap(), seen.iter().map(|p| p.y).min().unwrap(), seen.iter().map(|p| p.y).max().unwrap());
            eprintln!("{} child {} age? noble {:?} soldier {} labors {:?} body rest {} food {} meals {}", d.name, d.is_child(), d.noble, d.soldier, crate::dwarf::LABORS.iter().filter(|l| d.has(**l)).map(|l| l.short()).collect::<Vec<_>>(), d.body.needs_rest(), g.items.values().filter(|it| it.is_food()).count(), g.stock_of(ItemKind::Meal));
        }
        let id = g.dwarves[i].id;
        eprintln!("day {}: drinks {} wells {} alert {} civ {} burrows {:?}", g.tick / DAY, g.stock_of(ItemKind::Drink), g.working_wells().len(), g.alert, g.civ_alert, g.burrows.iter().map(|b| (b.name.clone(), b.members.len())).collect::<Vec<_>>());
        for _ in 0..40 {
            let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
            if !d.alive {
                eprintln!("  dead: {:?}", g.log.iter().rev().find(|l| l.color == (255, 60, 60)).map(|l| &l.text));
                break;
            }
            let w = &g.world;
            let wet = |q: Pos| crate::geom::DIRS8.iter().any(|&(dx, dy)| w.get(q.offset(dx, dy, 0)).map_or(false, |t| t.water > 0));
            let water = path::search(w, d.pos, 40_000, Mover::Dwarf, |q| wet(q)).map(|(p, path)| (p, path.len()));
            eprintln!("  t{} {} insane {} stress {} at {:?} thirst {} hunger {} fatigue {} task {:?} idle_cd {} path {} | water {:?} | burrowed {:?}", g.tick, d.name, d.insane, d.stress, d.pos, d.thirst, d.hunger, d.fatigue, d.task, d.idle_cd, d.path.len(), water, g.burrows.iter().filter(|b| b.members.contains(&id)).map(|b| b.name.clone()).collect::<Vec<_>>());
            for _ in 0..60 {
                g.step();
            }
        }
    }

    #[test]
    #[ignore]
    fn water_at_the_batch_sites() {
        for seed in 5000..5025u64 {
            let g = Game::new(seed);
            let e = g.embark;
            let w = &g.world;
            let wet = |q: Pos| crate::geom::DIRS8.iter().any(|&(dx, dy)| w.get(q.offset(dx, dy, 0)).map_or(false, |t| t.water >= 3));
            let found = path::search(w, e, 40_000, Mover::Dwarf, |q| wet(q));
            let water_tiles = w.tiles.iter().filter(|t| t.water >= 3).count();
            eprintln!("seed {} {:?} features {:?} water tiles {} reachable {}", seed, g.biome, crate::terrain::feature_names(w.features), water_tiles, found.map_or("no".to_string(), |(_, p)| format!("{} steps", p.len())));
        }
    }

    #[test]
    fn the_overseer_buries_its_dead() {
        let mut g = Game::new(5);
        // A plain round the wagon to dig into (the land itself may be a
        // valley the halls would have to cross).
        let e = g.embark;
        g.level_ground(e.x - 35, e.y - 25, e.x + 35, e.y + 25, e.z);
        g.set_overseer(Some(Style::Steady));
        for _ in 0..DAY * 3 {
            g.step();
        }
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.kill(&mut d, "was struck down");
        let name = d.name.clone();
        g.dwarves[0] = d;
        let mut days = 0;
        while !g.dwarves[0].rested && days < 40 {
            for _ in 0..DAY {
                g.step();
                g.creatures.retain(|_, c| c.tame || !c.hostile());
            }
            days += 1;
        }
        let o = g.overseer.as_ref().unwrap();
        assert!(o.rooms.iter().any(|r| r.kind == RoomUse::Tomb), "no tomb dug: {:?}", o.notes.iter().rev().take(8).collect::<Vec<_>>());
        assert!(g.dwarves[0].rested, "{} still lies unburied after {} days", name, days);
        eprintln!("{} buried after {} days", name, days);
        // With the tomb ready, the next to die is buried within days.
        let i = g.dwarves.iter().position(|d| d.alive && !d.soldier).unwrap();
        let mut d = std::mem::take(&mut g.dwarves[i]);
        g.kill(&mut d, "was struck down");
        let name = d.name.clone();
        g.dwarves[i] = d;
        let mut days = 0;
        while !g.dwarves[i].rested && days < 5 {
            for _ in 0..DAY {
                g.step();
                g.creatures.retain(|_, c| c.tame || !c.hostile());
            }
            days += 1;
        }
        let tomb = g.overseer.as_ref().unwrap().rooms.iter().find(|r| r.kind == RoomUse::Tomb).cloned().unwrap();
        let coffins: Vec<_> = g.buildings.values().filter(|b| b.kind == BuildingKind::Coffin).map(|b| (b.pos, b.built, b.occupant)).collect();
        assert!(g.dwarves[i].rested, "{} lies unburied {} days with the tomb ready ({:?} stage {}): coffins {:?}", name, days, tomb.a, tomb.stage, coffins);
        crate::game::watchdog::tests::assert_no_repeats(g.overseer.as_ref().unwrap());
    }

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
        assert!(g.zones.values().any(|z| z.tiles.iter().any(|p| p.z == base.z)), "nothing zoned below: {:?} {:?}", o.rooms.iter().map(|r| (r.kind, r.a, r.stage)).collect::<Vec<_>>(), o.notes);
    }

    #[test]
    #[ignore]
    fn style_trials() {
        let years: u64 = std::env::var("YEARS").ok().and_then(|s| s.parse().ok()).unwrap_or(2);
        let handles: Vec<_> = Style::ALL
            .iter()
            .enumerate()
            .filter(|(_, s)| std::env::var("ONLY").map_or(true, |o| o.eq_ignore_ascii_case(s.name())))
            .map(|(i, &s)| std::thread::spawn(move || run_trial(100 + i as u64, s, years, &Knobs::default())))
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
    fn find_slow_days() {
        use std::sync::atomic::Ordering::Relaxed;
        let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(500);
        let style = std::env::var("STYLE").ok().and_then(|s| Style::parse(&s)).unwrap_or(Style::Steady);
        let mut g = Game::new(seed);
        g.set_overseer(Some(style));
        let mut days: Vec<(u128, String)> = Vec::new();
        for day in 0..(YEAR * 2 / DAY) {
            let c0 = [crate::path::SEARCHES.load(Relaxed), crate::path::SEARCH_NODES.load(Relaxed), crate::path::ASTARS.load(Relaxed), crate::path::ASTAR_NODES.load(Relaxed)];
            let t = std::time::Instant::now();
            for _ in 0..DAY {
                g.step();
            }
            let ms = t.elapsed().as_millis();
            let c1 = [crate::path::SEARCHES.load(Relaxed), crate::path::SEARCH_NODES.load(Relaxed), crate::path::ASTARS.load(Relaxed), crate::path::ASTAR_NODES.load(Relaxed)];
            let flee = g.dwarves.iter().filter(|d| d.alive && d.task == Task::Flee).count();
            let hostile = g.creatures.values().filter(|c| c.alive && c.hostile()).count();
            days.push((
                ms,
                format!(
                    "day {:>3} {:>5}ms pop {:>2} searches {:>6} ({:>9} nodes) astar {:>6} ({:>8} nodes) desig {:>3} hostile {:>2} flee {:>2} alert {} creatures {}",
                    day,
                    ms,
                    g.alive_count(),
                    c1[0] - c0[0],
                    c1[1] - c0[1],
                    c1[2] - c0[2],
                    c1[3] - c0[3],
                    g.designations.len(),
                    hostile,
                    flee,
                    g.alert,
                    g.creatures.len()
                ),
            ));
            if g.alive_count() == 0 {
                break;
            }
        }
        let mut sorted: Vec<u128> = days.iter().map(|d| d.0).collect();
        sorted.sort();
        let median = sorted[sorted.len() / 2];
        eprintln!("median day {}ms over {} days", median, days.len());
        for (ms, line) in &days {
            if *ms > median * 3 && *ms > 60 {
                eprintln!("  SLOW {}", line);
            }
        }
        eprintln!("  typical: {}", days[days.len() / 2].1);

    }

    #[test]
    #[ignore]
    fn one_trial() {
        let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let style = std::env::var("STYLE").ok().and_then(|s| Style::parse(&s)).unwrap_or(Style::Steady);
        let years: u64 = std::env::var("YEARS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let t = run_trial(seed, style, years, &Knobs::default());
        eprintln!("{} {} {:.1}y {} | {}", t.style.name(), t.seed, t.years, t.verdict, t.epitaph);
    }

    #[test]
    fn styles_parse_and_trials_report() {
        assert_eq!(Style::parse("warlord"), Some(Style::Warlord));
        assert_eq!(Style::parse("nope"), None);
        let t = run_trial(7, Style::Reckless, 0, &Knobs::default());
        assert_eq!(t.verdict, "Endured");
        assert!(t.alive > 0);
    }

    #[test]
    fn a_hospital_before_the_first_siege() {
        // By the first siege's season, each careful style has its hospital
        // fitted, a doctor, and thread or cloth to dress wounds with, in a
        // fort that has grown far enough for its plan to reach the hospital
        // (the fifth room): the first from seed 5000 on with six rooms under
        // way by day 120. (Over seeds 5000-5024 the careful styles had six
        // by then in 97 of 100 forts.)
        for (style, first) in [(Style::Steady, 5000), (Style::Builder, 5000), (Style::Delver, 5000)] {
            let rooms = |g: &Game| g.overseer.as_ref().map_or(0, |o| o.rooms.iter().filter(|r| r.stage < 2).count());
            let (found, passed) = fort_where(style, first, 12, 120, |g| rooms(g) >= 6);
            let Some((seed, mut g)) = found else { panic!("{}: no fort from {} on grew to six rooms by day 120 (passed over {:?})", style.name(), first, passed) };
            while g.tick < 168 * DAY && g.alive_count() > 0 {
                g.step();
            }
            let o = g.overseer.as_ref().unwrap();
            let rooms: Vec<(RoomUse, u8)> = o.rooms.iter().map(|r| (r.kind, r.stage)).collect();
            assert!(o.rooms.iter().any(|r| r.kind == RoomUse::Hospital && r.stage == 1), "{} {}: no hospital fitted: {:?}", style.name(), seed, rooms);
            assert!(g.zones.values().any(|z| z.kind == ZoneKind::Stockpile(StockFilter::Medical)), "{} {}: no medical stockpile", style.name(), seed);
            assert!(g.has_doctor(), "{} {}: no doctor", style.name(), seed);
            let dressings = g.items.values().filter(|it| matches!(it.kind, ItemKind::Thread | ItemKind::Cloth)).count();
            assert!(dressings >= 2, "{} {}: {} thread or cloth", style.name(), seed, dressings);
        }
    }

    #[test]
    fn running_short_of_wood_for_ages_stays_cheap() {
        // The chop ring grows each time the logs run short; it used to grow
        // without end, checking millions of tiles off the map every day.
        let mut g = Game::new(5);
        let mut done = Vec::new();
        for _ in 0..200 {
            g.chop_further(&mut done);
        }
        let t = std::time::Instant::now();
        g.chop_further(&mut done);
        assert!(done.len() < 40, "the ring stops growing once it covers the map ({} rounds)", done.len());
        assert!(t.elapsed().as_millis() < 50, "a round costs {} ms", t.elapsed().as_millis());
        // And a box far off the map is no work at all.
        let t = std::time::Instant::now();
        assert_eq!(g.designate(Pos::new(-5000, -5000, 0), Pos::new(5000, 5000, 5), Desig::None), 0);
        assert!(t.elapsed().as_millis() < 50);
    }

    #[test]
    fn a_trial_game_is_the_trials_fort() {
        // Seed 5003 (Steady) has seen two small sieges and lost one dwarf
        // by day 262 of its year (schedule::same_story_after_refactor has
        // the whole year).
        let g = trial_game(5003, Style::Steady, 262);
        assert_eq!(g.tick, 262 * DAY);
        let deaths: Vec<(u64, &str)> = g.annals.deaths.iter().map(|d| (d.tick, d.name.as_str())).collect();
        assert_eq!(deaths, vec![]);
        let sieges: Vec<(u64, &str)> = g.annals.sieges.iter().map(|s| (s.tick, s.foe.as_str())).collect();
        assert_eq!(sieges, vec![(303845, "goblins")]);
        assert_eq!(g.overseer.as_ref().map(|o| o.style), Some(Style::Steady));
    }

    /// Whether open ground on the gate's level leads from the foot of the
    /// stair to the corridor behind the bridge without passing the doors
    /// (base+(1,0..1)) or the bridge (base+(2,0..1)).
    fn way_round_the_gate(g: &Game, base: Pos) -> bool {
        let blocked = |p: Pos| p.y - base.y >= 0 && p.y - base.y <= 1 && (p.x == base.x + 1 || p.x == base.x + 2);
        let goal = base.offset(6, 0, 0);
        let mut seen = std::collections::HashSet::from([base]);
        let mut todo = vec![base];
        while let Some(p) = todo.pop() {
            if p == goal {
                return true;
            }
            for &(dx, dy) in DIRS8.iter() {
                let q = p.offset(dx, dy, 0);
                if blocked(q) || seen.contains(&q) || !g.world.get(q).map_or(false, |t| !t.kind.solid()) {
                    continue;
                }
                seen.insert(q);
                todo.push(q);
            }
        }
        false
    }

    /// The gate is whole, its drawbridge linked to the lever, before the
    /// first army can come (day 168), whatever rock the halls are cut in
    /// (forts with no plain stone in theirs quarry it for the mechanisms),
    /// and the quarrying never cut a way round the doors and the bridge (it
    /// once dug the corridor's own walls, the closest stone). Forts in turn
    /// from seed 5000, a careful style each: the first eight, and as many
    /// more as it takes to see one that had to quarry. (The test once named
    /// its forts, a clay level here and a hematite one there, and each new
    /// way of making the land lost them. Over seeds 5000-5024 the careful
    /// styles linked the gate in all 100 forts, 19 of them after
    /// quarrying; at most one of the eight may fall short.)
    #[test]
    fn the_gate_is_whole_by_the_first_siege() {
        let styles = [Style::Steady, Style::Builder, Style::Delver, Style::Warlord];
        let mut short = Vec::new();
        let mut quarried = Vec::new();
        let done = |o: &Overseer| o.done.iter().filter(|d| !d.starts_with("farm") && !d.starts_with("chop")).cloned().collect::<Vec<_>>();
        for i in 0..20u64 {
            if i >= 8 && !quarried.is_empty() {
                break;
            }
            let (seed, style) = (5000 + i, styles[i as usize % 4]);
            let g = trial_game(seed, style, 168);
            let o = g.overseer.as_ref().unwrap();
            if o.did("quarry") {
                quarried.push(seed);
                assert!(o.did("linked"), "{} {}: quarried, but no linked drawbridge by day 168 (done {:?})", style.name(), seed, done(o));
            }
            if !o.did("linked") {
                short.push(format!("{} {} (alive {}, done {:?})", style.name(), seed, g.alive_count(), done(o)));
            }
            assert!(!way_round_the_gate(&g, o.base.unwrap()), "{} {}: a way round the gate by day 168", style.name(), seed);
        }
        assert!(short.len() <= 1, "no linked drawbridge by day 168: {:?}", short);
        assert!(!quarried.is_empty(), "no fort of twenty from 5000 on had to quarry for its gate");
    }

    /// One fort's halls at day DAYS, drawn: why its rooms stall.
    /// `SEED=5002 STYLE=steady DAYS=60 cargo test --release why_rooms_stall -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn why_rooms_stall() {
        let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(5002);
        let style = std::env::var("STYLE").ok().and_then(|s| Style::parse(&s)).unwrap_or(Style::Steady);
        let days: u64 = std::env::var("DAYS").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
        if let Ok(bx) = std::env::var("BOX") {
            let v: Vec<i32> = bx.split(',').filter_map(|s| s.parse().ok()).collect();
            let g = Game::new(seed);
            for y in v[1]..=v[3] {
                let row: Vec<String> = (v[0]..=v[2]).map(|x| { let t = g.world.t(Pos::new(x, y, v[4])); format!("{:?}{}", t.kind, if t.revealed { "r" } else { "" }) }).collect();
                eprintln!("y{} {}", y, row.join(" "));
            }
        }
        let mut g = trial_game(seed, style, days);
        let mut o = g.overseer.take().unwrap();
        let b = o.base.unwrap();
        let e = g.embark;
        eprintln!("embark {:?} top {:?} base {:?} layout {:?} level {} planned {} wants {:?} taken {:?} plan_len {}", e, o.top, b, o.layout, o.level, o.planned, o.wants, o.plan_taken, o.plan_len);
        eprintln!("rooms {:?}", o.rooms.iter().map(|r| (r.kind, r.a, r.stage)).collect::<Vec<_>>());
        eprintln!("caverns {:?} aquifer {} water_level {}", g.world.caverns, g.world.aquifer, g.world.water_level);
        for d in g.dwarves.iter().filter(|d| d.alive) {
            eprintln!("  {} mining {} task {:?} pos {:?}", d.name, d.has(Labor::Mining), d.task, d.pos);
        }
        eprintln!("picks {} designations {} notes:", g.items.values().filter(|it| it.kind == ItemKind::Pick).count(), g.designations.len());
        for n in o.notes.iter().rev().take(25).rev() {
            eprintln!("  d{} {}", n.0 / DAY, n.1);
        }
        if std::env::var("SURFACE").is_ok() {
            // The whole map's surface, by height relative to the wagon (a-z below, A-Z above, ~ water, E wagon, S stair).
            let mut s = String::new();
            for y in 0..g.world.h {
                for x in 0..g.world.w {
                    let z = g.world.surface_z(x, y);
                    let t = g.world.t(Pos::new(x, y, z));
                    let d = z - e.z;
                    s.push(if (x, y) == (e.x, e.y) { '@' } else if Some((x, y)) == o.top.map(|t| (t.x, t.y)) { 'S' } else if t.water > 0 { '~' } else if d == 0 { '.' } else if d > 0 { (b'A' + (d.min(25) as u8) - 1) as char } else { (b'a' + ((-d).min(26) as u8) - 1) as char });
                }
                s.push(char::from(10u8));
            }
            eprintln!("{}", s);
        }
        // Round the top of the stair: height of each column against the top (digits; letters for lower), W a wall built, w one planned, D a door.
        if let Some(t0) = o.top {
            for y in t0.y - 6..=t0.y + 6 {
                let mut row = String::new();
                for x in t0.x - 6..=t0.x + 6 {
                    if !g.world.in_bounds(Pos::new(x, y, 0)) { row.push(' '); continue; }
                    let z = g.world.surface_z(x, y);
                    let p = Pos::new(x, y, t0.z);
                    let c = if (x, y) == (t0.x, t0.y) { 'X' } else if g.constructions.values().any(|c| c.pos.x == x && c.pos.y == y) { 'w' } else if g.world.get(p).map_or(false, |t| t.built_from.is_some()) { 'W' } else if g.building_at(p) == Some(BuildingKind::Door) { 'D' } else {
                        let d = z - t0.z;
                        if d >= 0 { (b'0' + d.min(9) as u8) as char } else { (b'a' + (-d).min(25) as u8 - 1) as char }
                    };
                    row.push(c);
                }
                eprintln!("  top {}", row);
            }
        }
        // What a dwarf can walk to from the foot of the stair on its level.
        let mut reach = std::collections::HashSet::from([b]);
        let mut todo = vec![b];
        while let Some(p) = todo.pop() {
            for &(dx, dy) in DIRS8.iter() {
                let q = p.offset(dx, dy, 0);
                if !reach.contains(&q) && g.world.walkable(q) {
                    reach.insert(q);
                    todo.push(q);
                }
            }
        }
        for z in [b.z + 1, b.z, b.z - 2] {
            eprintln!("== z {} (base.z {:+})  # rock  . dug  , natural floor  ' ' open sky/air  ~ water  A aquifer  X bedrock  + desig  o open unrevealed  s soil", z, z - b.z);
            let mut s = String::new();
            for y in (b.y - 28).max(0)..(b.y + 29).min(g.world.h) {
                for x in (b.x - 4).max(0)..(b.x + 80).min(g.world.w) {
                    let p = Pos::new(x, y, z);
                    let t = g.world.t(p);
                    s.push(if p == b { '@' } else if t.water > 0 {
                        '~'
                    } else if t.aquifer {
                        'A'
                    } else if t.desig != Desig::None {
                        '+'
                    } else if t.kind.solid() {
                        if t.mat == Mat::Bedrock { 'X' } else if !t.mat.is_building_stone() { 's' } else { '#' }
                    } else if t.dug {
                        if reach.contains(&p) { '.' } else { ':' }
                    } else if t.kind.has_floor() {
                        ','
                    } else if t.revealed {
                        ' '
                    } else {
                        'o'
                    });
                }
                s.push(char::from(10u8));
            }
            eprintln!("{}", s);
        }
        let kind = plan(o.style).get(o.planned).copied().unwrap_or(RoomUse::Dining);
        let r = g.plan_room(&mut o, kind);
        eprintln!("plan_room({:?}) now: {:?}", kind, r.map(|r| (r.a, r.b)));
    }

    /// How reliably the overseer builds on whatever land it gets: for each
    /// seed (SEED0.., SEEDS of them) and style, its rooms at day 120 and its
    /// gate at day 168. `cargo test --release reliability_table -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn reliability_table() {
        let seed0: u64 = std::env::var("SEED0").ok().and_then(|s| s.parse().ok()).unwrap_or(5000);
        let n: u64 = std::env::var("SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(25);
        let threads: usize = std::env::var("THREADS").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
        let styles = [Style::Steady, Style::Builder, Style::Delver, Style::Warlord, Style::Reckless];
        let seeds: Vec<u64> = match std::env::var("SEEDLIST") {
            Ok(l) => l.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
            Err(_) => (seed0..seed0 + n).collect(),
        };
        let only = std::env::var("STYLE").ok().and_then(|s| Style::parse(&s));
        let jobs: Vec<(u64, Style)> = seeds.iter().flat_map(|&s| styles.iter().filter(|&&st| only.map_or(true, |o| o == st)).map(move |&st| (s, st))).collect();
        let next = std::sync::atomic::AtomicUsize::new(0);
        let rows = std::sync::Mutex::new(Vec::new());
        std::thread::scope(|sc| {
            for _ in 0..threads {
                sc.spawn(|| loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(&(seed, style)) = jobs.get(i) else { break };
                    let mut g = Game::new(seed);
                    let (piece, main) = crate::terrain_gen::connect::surface_pieces(&g.world);
                    let walk = piece.iter().filter(|&&p| p != u32::MAX).count();
                    let cut = piece.iter().filter(|&&p| p != u32::MAX && p != main).count();
                    g.set_overseer(Some(style));
                    let mut at120 = (0, 0);
                    while g.tick < 168 * DAY && g.alive_count() > 0 {
                        g.step();
                        if g.tick == 120 * DAY {
                            if let Some(o) = g.overseer.as_ref() {
                                at120 = (o.rooms.iter().filter(|r| r.stage != 2).count(), o.rooms.iter().filter(|r| r.stage == 1).count());
                            }
                        }
                    }
                    let row = match g.overseer.as_ref().filter(|o| o.base.is_some()) {
                        None => format!("{} {:<8} NO BASE  alive {}", seed, style.name(), g.alive_count()),
                        Some(o) => {
                            let base = o.base.unwrap();
                            let gaveup = o.rooms.iter().filter(|r| r.stage == 2).count();
                            let fitted = o.rooms.iter().filter(|r| r.stage == 1).count();
                            let bridges = g.gate_bridges(base).iter().filter(|id| g.buildings.get(id).map_or(false, |b| b.built)).count();
                            let hosp = o.rooms.iter().any(|r| r.kind == RoomUse::Hospital && r.stage == 1);
                            let last = o.notes.iter().rev().find(|n| n.1.contains("Dug out") || n.1.contains("Gave up") || n.1.contains("bedrooms")).map_or(String::new(), |n| format!("d{} {}", n.0 / DAY, n.1));
                            format!(
                                "{} {:<8} {:<10} z{:<2} rooms120 {:>2}/{:<2} rooms168 {:>2}/{:<2} gaveup {} lvl {} | gate {} bridge {}/2 linked {} roundgate {} | hosp {} | cut {:.1}% alive {} | quarry {} mech {} shop {} stone {} | base {},{} of {}x{} | corr {:.2} parts {} | medpile {} doctor {} dressings {} farm {} | deaths {:?} | {}",
                                seed,
                                style.name(),
                                format!("{:?}", o.layout),
                                base.z,
                                at120.0,
                                at120.1,
                                o.rooms.len() - gaveup,
                                fitted,
                                gaveup,
                                o.level,
                                o.did("gate") as u8,
                                bridges,
                                o.did("linked") as u8,
                                way_round_the_gate(&g, base) as u8,
                                hosp as u8,
                                cut as f64 * 100.0 / walk.max(1) as f64,
                                g.alive_count(),
                                o.did("quarry") as u8,
                                g.stock_of(ItemKind::Mechanism),
                                g.buildings.values().any(|b| b.kind == BuildingKind::Mechanic && b.built) as u8,
                                g.items.values().filter(|it| it.kind == ItemKind::Boulder && it.mat.is_building_stone() && it.free()).count(),
                                base.x,
                                base.y,
                                g.world.w,
                                g.world.h,
                                g.dug_share(base.offset(1, 0, 0), base.offset(14, 1, 0)),
                                [base.offset(2, 0, 0), base.offset(2, 1, 0), base.offset(3, 1, 0)]
                                    .iter()
                                    .map(|&p| {
                                        let t = g.world.t(p);
                                        format!("{:?}{}/{:?}", t.kind, if t.building != 0 { "+b" } else { "" }, g.building_at(p))
                                    })
                                    .collect::<Vec<_>>()
                                    .join(","),
                                g.zones.values().any(|z| z.kind == ZoneKind::Stockpile(StockFilter::Medical)) as u8,
                                g.has_doctor() as u8,
                                g.items.values().filter(|it| matches!(it.kind, ItemKind::Thread | ItemKind::Cloth)).count(),
                                g.zones.values().filter(|z| z.kind == ZoneKind::Farm).map(|z| z.tiles.len()).sum::<usize>(),
                                g.annals.deaths.iter().fold(BTreeMap::<String, u32>::new(), |mut m, d| {
                                    *m.entry(format!("{}/{}/d{}", d.cause, d.killer.clone().unwrap_or_default(), d.tick / DAY / 30 * 30)).or_default() += 1;
                                    m
                                }),
                                last
                            )
                        }
                    };
                    eprintln!("{}", row);
                    rows.lock().unwrap().push((seed, style.name(), row));
                });
            }
        });
        let mut rows = rows.into_inner().unwrap();
        rows.sort();
        eprintln!("==== sorted");
        for r in rows {
            eprintln!("{}", r.2);
        }
    }

    /// A saved fort's overseer, looked over: why it stalls, and whether it
    /// gets going again over DAYS days (MAP draws the levels round the
    /// stair; PEACE keeps enemies away). With PEACE and DAYS >= 40 it checks
    /// the fort got going: a room fitted out, the dead buried, no line said
    /// twice in half a season.
    /// `FORT=path/to/save.json DAYS=40 PEACE=1 cargo test --release stuck_save -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn stuck_save() {
        let path = std::env::var("FORT").unwrap();
        let days: u64 = std::env::var("DAYS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
        let mut g: Game = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        g.after_load();
        let show = |g: &Game| {
            let o = g.overseer.as_ref().unwrap();
            let idle = g.dwarves.iter().filter(|d| d.alive && matches!(d.task, crate::dwarf::Task::Idle)).count();
            eprintln!(
                "day {} style {:?} layout {:?} level {} base {:?} top {:?} planned {} wants {:?} plan_len {} taken {} anchors {} rooms {} (fitted {}) desig {} alive {} idle {} buildings {}",
                g.tick / DAY, o.style, o.layout, o.level, o.base, o.top, o.planned, o.wants, o.plan_len, o.plan_taken.len(), o.anchors.len(),
                o.rooms.len(), o.rooms.iter().filter(|r| r.stage == 1).count(), g.designations.len(), g.alive_count(), idle, g.buildings.len()
            );
        };
        show(&g);
        let start = g.tick;
        {
            let o = g.overseer.as_ref().unwrap();
            eprintln!("done {:?}", o.done.iter().filter(|d| !d.starts_with("farm")).collect::<Vec<_>>());
            eprintln!("rooms {:?}", o.rooms.iter().map(|r| (r.kind, r.a, r.stage)).collect::<Vec<_>>());
            eprintln!("caverns {:?} aquifer {} water_level {} map {}x{}x{}", g.world.caverns, g.world.aquifer, g.world.water_level, g.world.w, g.world.h, g.world.d);
            let mut last = String::new();
            for n in &o.notes {
                if n.1 != last {
                    eprintln!("  d{} {}", n.0 / DAY, n.1);
                }
                last = n.1.clone();
            }
        }
        // The levels round the foot of the stair, drawn.
        if std::env::var("MAP").is_ok() {
            let o = g.overseer.as_ref().unwrap();
            let b = o.base.unwrap();
            for z in [b.z + 1, b.z, b.z - 1, b.z - 2] {
                eprintln!("== z {} (base.z {:+})  # rock . dug/floor ' ' air ~ water A aquifer X bedrock + desig s soil", z, z - b.z);
                let mut s = String::new();
                for y in 0..g.world.h {
                    for x in 0..g.world.w {
                        let p = Pos::new(x, y, z);
                        let t = g.world.t(p);
                        s.push(if p == b { '@' } else if t.water > 0 { '~' } else if t.aquifer { 'A' } else if t.desig != Desig::None { '+' } else if t.kind.solid() {
                            if t.mat == Mat::Bedrock { 'X' } else if !t.mat.is_building_stone() { 's' } else { '#' }
                        } else if t.kind.has_floor() { '.' } else { ' ' });
                    }
                    s.push(char::from(10u8));
                }
                eprintln!("{}", s);
            }
        }
        // The dead: where each lies, and whether a hauler could get there.
        {
            let b = g.overseer.as_ref().unwrap().base.unwrap();
            for it in g.items.values().filter(|it| it.kind == ItemKind::Corpse) {
                let ours = g.burial_corpse(it);
                let path = path::astar(&g.world, b, it.pos, 40_000, Mover::Dwarf).is_some();
                let t = g.world.t(it.pos);
                eprintln!("corpse {:?} ours {} at {:?} free {} forbidden {} claimed {:?} carried {:?} reach {} water {} outside {}", it.label, ours, it.pos, it.free(), it.forbidden, it.claimed_by, it.carried_by, path, t.water, g.world.outside(it.pos));
            }
            for c in g.buildings.values().filter(|c| c.kind == BuildingKind::Coffin) {
                eprintln!("coffin {:?} built {} occupant {:?} worker {:?}", c.pos, c.built, c.occupant, c.worker);
            }
            eprintln!("haulers {} unrested dead {:?}", g.dwarves.iter().filter(|d| d.alive && d.has(Labor::Hauling)).count(), g.dwarves.iter().filter(|d| !d.alive && !d.rested).map(|d| (d.name.clone(), d.ghost)).collect::<Vec<_>>());
        }
        // Each level the planner would look at, and what stops it there.
        {
            let mut g2: Game = serde_json::from_str(&std::fs::read_to_string(std::env::var("FORT").unwrap()).unwrap()).unwrap();
            g2.after_load();
            let o = g2.overseer.take().unwrap();
            let b = o.base.unwrap();
            let roof = g2.world.caverns.iter().map(|c| c.1).max().unwrap_or(0);
            for k in 0..=o.level + 2 {
                let z = Game::level_z(&o, k);
                let bp = o.layout.planned().then(|| g2.plan_slot(&mut o.clone(), RoomUse::Hospital, k).map(|r| r.0.a));
                eprintln!("level {} z {} (cavern roof {}) base.z {} plan_slot {:?}", k, z, roof, b.z, bp);
            }
            for kind in [RoomUse::Hospital, RoomUse::Tomb, RoomUse::Workshops] {
                let r = g2.plan_room(&mut o.clone(), kind);
                eprintln!("plan_room({:?}): {:?}", kind, r.map(|r| (r.a, r.b)));
            }
        }
        {
            let o = g.overseer.as_ref().unwrap();
            eprintln!("hostiles on the map {} (siege {})", g.creatures.values().filter(|c| c.alive && c.hostile()).count(), g.creatures.values().filter(|c| c.alive && c.hostile() && c.siege).count());
            eprintln!("a new stair: {:?}; the first choice now: {:?}", g.stair_site(o.style, o.top), g.stair_site(o.style, None));
        }
        // (PEACE: no enemies, to see the overseer's building alone.)
        let peace = std::env::var("PEACE").is_ok();
        for day in 0..days {
            for _ in 0..DAY {
                g.step();
                if peace {
                    g.creatures.retain(|_, c| c.tame || !c.hostile());
                    g.incoming = None;
                }
            }
            if day < 5 || day % 5 == 4 || day + 1 == days {
                show(&g);
            }
        }
        {
            let picks = g.items.values().filter(|it| it.kind == ItemKind::Pick).count();
            let miners: Vec<String> = g.dwarves.iter().filter(|d| d.alive && d.has(Labor::Mining)).map(|d| format!("{} {:?}", d.name, d.task)).collect();
            eprintln!("picks {} miners {:?}", picks, miners);
            let o = g.overseer.as_ref().unwrap();
            let b = o.base.unwrap();
            let mut kinds = BTreeMap::<String, (usize, usize)>::new();
            for &p in g.designations.iter() {
                let t = g.world.t(p);
                let reach = g.work_targets(p).into_iter().any(|_| true) && crate::geom::DIRS8.iter().any(|&(dx, dy)| path::astar(&g.world, b, p.offset(dx, dy, 0), 20_000, Mover::Dwarf).is_some());
                let e = kinds.entry(format!("{:?} z{}", t.desig, p.z)).or_default();
                e.0 += 1;
                if reach {
                    e.1 += 1;
                }
            }
            eprintln!("designations (all, reachable from the base): {:?}", kinds);
            eprintln!("rooms {:?}", o.rooms.iter().map(|r| (r.kind, r.a, r.stage, g.room_dug(r))).collect::<Vec<_>>());
            eprintln!(
                "unrested dead {} coffins {} built {} filled {} coffin items {} logs {}",
                g.dwarves.iter().filter(|d| !d.alive && !d.rested).count(),
                g.buildings.values().filter(|b| b.kind == BuildingKind::Coffin).count(),
                g.buildings.values().filter(|b| b.kind == BuildingKind::Coffin && b.built).count(),
                g.buildings.values().filter(|b| b.kind == BuildingKind::Coffin && b.occupant.is_some()).count(),
                g.stock_of(ItemKind::Coffin),
                g.stock_of(ItemKind::Log)
            );
        }
        for d in g.annals.deaths.iter().filter(|d| d.tick >= start) {
            eprintln!("  died d{} {} {} {:?} outside {}", d.tick / DAY, d.name, d.cause, d.killer, d.outside);
        }
        let o = g.overseer.as_ref().unwrap();
        let mut last = String::new();
        for n in o.notes.iter().rev().take(30).rev() {
            if n.1 != last {
                eprintln!("  d{} {}", n.0 / DAY, n.1);
            }
            last = n.1.clone();
        }
        if peace && days >= 40 {
            assert!(o.rooms.iter().any(|r| r.stage == 1), "no room fitted out in {} days", days);
            assert!(g.dwarves.iter().all(|d| d.alive || d.rested), "the dead still lie unburied");
            let mut since = o.clone();
            since.notes.retain(|n| n.0 >= start);
            crate::game::watchdog::tests::assert_no_repeats(&since);
        }
    }
}
