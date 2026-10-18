//! The thing in the night: a named horror as a mystery. Some time after the
//! first year, something comes to live near the fort. Nobody sees it; the
//! dwarves give it a name ("the Pale Drinker") and notice its signs: a goat
//! drained of blood in the pasture, tracks as wide as a cartwheel, a hauler
//! gone missing and found at dawn, torn and senseless. The sheriff (or the
//! most learned dwarf there is) looks into it and, clue by clue, finds what
//! it can't bear: gold, fire, running water or a warding engraving. Then the
//! hunt: a bold overseer sends the squad out to its lair at once, a steady
//! one once the bane is ready, a cautious one keeps everyone indoors and
//! waits behind the walls; whoever doesn't go to it, it comes to in the end.
//! Its look, powers, name and bane come from the world's seed and the site,
//! and it's all saved with the fort (in `Horrors`).

use super::horrors::{pick_powers, Power};
use super::military::SquadOrder;
use super::overseer::{Overseer, Style};
use super::personality::Trait;
use super::positions::Position;
use super::*;
use crate::body::Body;
use crate::creature::{Creature, Species};
use serde::{Deserialize, Serialize};

/// What the thing can't bear.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bane {
    Gold,
    Fire,
    RunningWater,
    Engraving,
}

impl Bane {
    pub const ALL: [Bane; 4] = [Bane::Gold, Bane::Fire, Bane::RunningWater, Bane::Engraving];

    pub fn name(self) -> &'static str {
        match self {
            Bane::Gold => "gold",
            Bane::Fire => "fire",
            Bane::RunningWater => "running water",
            Bane::Engraving => "a warding engraving",
        }
    }

    /// The clues that point to it, in the order they're found.
    fn clues(self) -> [&'static str; 3] {
        match self {
            Bane::Gold => [
                "its tracks stop dead where a vein of gold shows in the rock",
                "the goat it left had a gold ring in its ear, and the ring was not touched",
                "the old tales of the hills say such things are burnt by gold",
            ],
            Bane::Fire => [
                "its trail goes wide round every hearth and ashpit",
                "the night the brazier was lit, nothing was taken",
                "there is scorched hair on a thorn where it fled a campfire",
            ],
            Bane::RunningWater => [
                "its tracks run along the brook for a mile and never cross it",
                "the hauler was found on the far bank, and it had not followed",
                "the old songs say such things cannot pass running water",
            ],
            Bane::Engraving => [
                "it clawed at every rough stone by the gate and left the carved ones alone",
                "there are scratches before an engraving, as if something would not come closer",
                "an old carving in a ruin shows the same beast, cowering from a warding sign",
            ],
        }
    }

    /// What bites it ("the gold").
    fn bites(self) -> &'static str {
        match self {
            Bane::Gold => "the gold",
            Bane::Fire => "the fire",
            Bane::RunningWater => "the running water",
            Bane::Engraving => "the warding sign",
        }
    }

    /// What the fort does to arm the hunters with it.
    fn armed_text(self) -> &'static str {
        match self {
            Bane::Gold => "gilded the hunters' blades",
            Bane::Fire => "bound torches for the hunters",
            Bane::RunningWater => "filled flasks with running water for the hunters",
            Bane::Engraving => "cut warding signs into the hunters' shields",
        }
    }

    /// What's missing when the fort can't.
    fn lacking(self) -> &'static str {
        match self {
            Bane::Gold => "there is no gold in the fort to gild the blades",
            Bane::Fire => "there is no wood to bind torches",
            Bane::RunningWater => "there is no running water to draw",
            Bane::Engraving => "there is no engraving in the fort to copy",
        }
    }
}

/// How far the tale has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Stage {
    /// The dwarves notice its signs.
    Signs,
    /// Someone is looking into it.
    Investigating,
    /// Its bane and its lair are known.
    Known,
    /// The squad has gone out after it.
    Hunted,
    /// It comes to the fort.
    Coming,
    /// It's dead.
    Slain,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Haunt {
    /// What the dwarves call it ("the Pale Drinker").
    pub nickname: String,
    /// Its own name, as a cave horror's ("Vexoth").
    pub name: String,
    /// "A spindly spider of shadow with many eyes."
    pub look: String,
    pub powers: Vec<Power>,
    pub bane: Bane,
    /// Where it lairs (out on the surface).
    pub lair: Pos,
    pub stage: Stage,
    /// When it began, and when this stage began.
    pub began: u64,
    pub since: u64,
    /// What the dwarves have noticed, in order.
    #[serde(default)]
    pub signs: Vec<String>,
    /// Who is looking into it.
    #[serde(default)]
    pub investigator: Option<u32>,
    #[serde(default)]
    pub clues: u8,
    /// It, on the map (once hunted or coming).
    #[serde(default)]
    pub creature: Option<u32>,
    /// The hunters carry its bane.
    #[serde(default)]
    pub armed: bool,
    /// The squad out after it (its id).
    #[serde(default)]
    pub squad: Option<u32>,
    /// Signs the overseer has heard of.
    #[serde(default)]
    pub heard: usize,
    /// It has felt its bane in this fight (told once).
    #[serde(default)]
    pub burnt: bool,
}

const ADJ: [&str; 10] = ["Pale", "Grey", "Hollow", "Weeping", "Hungry", "Long", "Silent", "Black", "Crooked", "Night"];
const NOUN: [&str; 10] = ["Drinker", "Walker", "Hound", "Widow", "Shepherd", "Stalker", "Mother", "Grinner", "Thing", "Hunter"];
const SYL: [&str; 12] = ["oz", "ul", "thra", "gim", "vex", "oth", "rhu", "zal", "mor", "quo", "yth", "nug"];
const SIZE: [&str; 5] = ["towering", "hulking", "bloated", "spindly", "gaunt"];
const SHAPE: [&str; 6] = ["serpent", "spider", "beetle", "lizard", "bat", "hound"];
const STUFF: [&str; 6] = ["bone", "slime", "iron", "ash", "shadow", "crystal"];
const MARK: [&str; 5] = ["many eyes", "a gaping maw", "barbed tentacles", "glowing veins", "no eyes at all"];

/// Clues it takes to find the bane.
const CLUES: u8 = 3;

fn cap(s: &str) -> String {
    super::combat::cap(s)
}

/// "north", "south-east": which way `to` lies from `from`.
fn compass(from: Pos, to: Pos) -> &'static str {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let ns = if dy * 2 < -dx.abs() { "north" } else if dy * 2 > dx.abs() { "south" } else { "" };
    let ew = if dx * 2 < -dy.abs() { "west" } else if dx * 2 > dy.abs() { "east" } else { "" };
    match (ns, ew) {
        ("", "") => "near",
        ("", e) => e,
        (n, "") => n,
        ("north", "east") => "north-east",
        ("north", _) => "north-west",
        (_, "east") => "south-east",
        _ => "south-west",
    }
}

impl Game {
    /// The thing this site's world would send (the same every time).
    pub(super) fn make_haunt(&self) -> Haunt {
        let mut r = self.horror_rng(0, 0x4A_0E7);
        let nickname = format!("the {} {}", r.pick(&ADJ), r.pick(&NOUN));
        let n = r.range(2, 4);
        let name = cap(&(0..n).map(|_| *r.pick(&SYL)).collect::<String>());
        let look = format!("A {} {} of {} with {}.", r.pick(&SIZE), r.pick(&SHAPE), r.pick(&STUFF), r.pick(&MARK));
        let powers = pick_powers(&mut r, &look, 1, 2);
        // A fire-breather that feared fire would sear itself on its own breath.
        let banes: Vec<Bane> = Bane::ALL.into_iter().filter(|b| *b != Bane::Fire || !powers.contains(&Power::Fire)).collect();
        let bane = *r.pick(&banes);
        let lair = self.haunt_lair(&mut r);
        Haunt { nickname, name, look, powers, bane, lair, stage: Stage::Signs, began: 0, since: 0, signs: Vec::new(), investigator: None, clues: 0, creature: None, armed: false, squad: None, heard: 0, burnt: false }
    }

    /// Open ground out on the surface, a long walk from the wagon.
    fn haunt_lair(&self, r: &mut Rng) -> Pos {
        let e = self.embark;
        let w = &self.world;
        for _ in 0..400 {
            let a = r.f64() * std::f64::consts::TAU;
            let d = r.range(16, 30) as f64;
            let (x, y) = (e.x + (a.cos() * d) as i32, e.y + (a.sin() * d) as i32);
            if x < 2 || y < 2 || x >= w.w - 2 || y >= w.h - 2 {
                continue;
            }
            let z = w.surface_z(x, y);
            for p in [Pos::new(x, y, z), Pos::new(x, y, z + 1)] {
                if w.in_bounds(p) && w.walkable(p) && w.outside(p) && w.get(p).map_or(false, |t| t.water == 0) {
                    return p;
                }
            }
        }
        e
    }

    /// Whether something haunts this site at all (most do; the same answer
    /// every time).
    fn haunted_site(&self) -> bool {
        self.horror_rng(0, 0x4A_77).chance(self.knob("haunt.odds", 0.75))
    }

    /// The day's own dice for the tale (the fort's are left alone).
    fn haunt_dice(&self, salt: u64) -> Rng {
        self.horror_rng((self.tick / DAY) as u32, salt)
    }

    /// Daily (schedule.rs): it comes, leaves its signs, is looked into and
    /// found out, and in the end comes to the fort if nobody goes to it.
    pub(super) fn haunt_daily(&mut self) {
        let Some(h) = self.horrors.haunt.clone() else {
            if self.tick >= self.knob("haunt.after", YEAR as f64) as u64 && self.haunted_site() && self.haunt_dice(0x4A_D1).chance(self.knob("haunt.chance", 0.04)) {
                self.begin_haunt();
            }
            return;
        };
        let mut r = self.haunt_dice(0x4A_51);
        match h.stage {
            Stage::Signs | Stage::Investigating => {
                let odds = if h.stage == Stage::Signs { 0.4 } else { 0.2 };
                if r.chance(odds) {
                    self.haunt_sign(&mut r);
                }
                let signs = self.horrors.haunt.as_ref().map_or(0, |h| h.signs.len());
                if signs >= 2 {
                    self.investigate(&mut r);
                }
                // Left alone long enough, it comes.
                if self.tick > h.began + (self.knob("haunt.patience", 40.0) * DAY as f64) as u64 {
                    self.haunt_comes();
                }
            }
            Stage::Known => {
                // Gold or wood may have come in since.
                self.arm_against_haunt(false);
                if self.tick > h.since + (self.knob("haunt.comes", 12.0) * DAY as f64) as u64 {
                    self.haunt_comes();
                }
            }
            Stage::Hunted => {
                self.arm_against_haunt(false);
                // Not found in three days: it turns on the fort.
                if self.tick > h.since + (self.knob("haunt.hunt_days", 3.0) * DAY as f64) as u64 {
                    self.haunt_comes();
                }
            }
            Stage::Coming | Stage::Slain => {}
        }
    }

    /// The dwarves begin to whisper of it.
    pub(super) fn begin_haunt(&mut self) {
        let mut h = self.make_haunt();
        h.began = self.tick;
        h.since = self.tick;
        let text = format!("The dwarves whisper of something out past the fields at night. They call it {}.", h.nickname);
        let at = h.lair;
        self.horrors.haunt = Some(h);
        self.count("haunt.begun", 1);
        self.msg(&text, (200, 170, 230));
        self.spotlight("Something in the night", &text, Some(at), (200, 170, 230));
    }

    /// One of its signs: livestock drained, tracks, a hauler gone missing.
    fn haunt_sign(&mut self, r: &mut Rng) {
        let Some(h) = self.horrors.haunt.clone() else { return };
        let way = compass(self.embark, h.lair);
        let nick = h.nickname.clone();
        let tick = self.tick;
        // Livestock out in the open, not a pet or a war dog.
        let beasts: Vec<u32> = self.creatures.values().filter(|c| c.alive && c.tame && c.pet_of.is_none() && c.owner.is_none() && self.world.outside(c.pos)).map(|c| c.id).collect();
        // A hauler alone outside (once it's grown bold).
        let haulers: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.soldier && !d.is_child() && !d.insane && self.world.outside(d.pos)
            })
            .collect();
        let pick = r.range(0, 3);
        let text = if pick == 0 && !beasts.is_empty() {
            let id = *r.pick(&beasts);
            let mut c = self.creatures.remove(&id).unwrap();
            let what = c.species.info().name;
            c.body.dead = true;
            c.alive = false;
            self.creature_died(c);
            format!("A {} was found at dawn drained of blood, two neat holes in its throat. The dwarves blame {}.", what, nick)
        } else if pick == 1 && h.signs.len() >= 1 && !haulers.is_empty() {
            let i = *r.pick(&haulers);
            let mut d = std::mem::take(&mut self.dwarves[i]);
            d.body.wound(crate::body::Part::LeftArm, 3, true);
            d.think(&format!("was carried off by {} and remembers nothing", nick), 30, tick);
            let name = d.called();
            self.dwarves[i] = d;
            format!("{} went missing while hauling and was found at dawn by the {} edge, torn and senseless, babbling of {}.", name, way, nick)
        } else {
            let sign = *r.pick(&["tracks as wide as a cartwheel", "a trail of slime that smokes in the frost", "claw marks gouged high in the bark", "a ring of dead grass, as if something lay there"]);
            format!("{} round the fields, leading off to the {}: {} has been near.", cap(sign), way, nick)
        };
        if let Some(h) = self.horrors.haunt.as_mut() {
            h.signs.push(text.clone());
        }
        self.count("haunt.signs", 1);
        self.msg(&text, (200, 170, 230));
        self.gossip(text);
    }

    /// Who looks into it: the sheriff, else the bookkeeper or the manager,
    /// else the eldest grown dwarf in their right mind.
    fn investigator(&self) -> Option<(u32, String)> {
        for (p, role) in [(Position::Sheriff, "the sheriff"), (Position::Bookkeeper, "the bookkeeper"), (Position::Manager, "the manager")] {
            if let Some(d) = self.holder(p) {
                return Some((d.id, role.to_string()));
            }
        }
        self.dwarves.iter().filter(|d| d.alive && !d.insane && !d.is_child()).max_by_key(|d| (d.age.unwrap_or(0), std::cmp::Reverse(d.id))).map(|d| (d.id, "the eldest of the fort".to_string()))
    }

    /// A day of looking into it: someone takes it up, and a clue may turn up.
    fn investigate(&mut self, r: &mut Rng) {
        let Some(h) = self.horrors.haunt.clone() else { return };
        let live = h.investigator.and_then(|id| self.dwarves.iter().find(|d| d.id == id && d.alive && !d.insane));
        let who = match live {
            Some(d) => d.id,
            None => {
                let Some((id, role)) = self.investigator() else { return };
                let name = self.dwarves.iter().find(|d| d.id == id).map(|d| d.called()).unwrap_or_default();
                let text = format!("{}, {}, has begun to look into {}.", name, role, h.nickname);
                self.msg(&text, (200, 200, 140));
                if let Some(h) = self.horrors.haunt.as_mut() {
                    h.investigator = Some(id);
                    if h.stage == Stage::Signs {
                        h.stage = Stage::Investigating;
                        h.since = self.tick;
                    }
                }
                return;
            }
        };
        if !r.chance(self.knob("haunt.clue", 0.45)) {
            return;
        }
        let k = h.clues as usize;
        let clue = h.bane.clues()[k.min(2)];
        let name = self.dwarves.iter().find(|d| d.id == who).map(|d| d.called()).unwrap_or_default();
        let text = format!("{} has found that {}.", name, clue);
        self.msg(&text, (200, 200, 140));
        if let Some(h) = self.horrors.haunt.as_mut() {
            h.clues += 1;
        }
        if h.clues + 1 >= CLUES {
            self.haunt_found_out(Some(who));
        }
    }

    /// Its bane and lair are known: told, written in the legends, the
    /// hunters armed with it if the fort can, and (in a player's fort) the
    /// thing itself seen at its lair.
    pub(super) fn haunt_found_out(&mut self, by: Option<u32>) {
        let Some(h) = self.horrors.haunt.clone() else { return };
        if h.stage >= Stage::Known {
            return;
        }
        let fort = self.fortress_name.clone();
        let who = by.and_then(|id| self.dwarves.iter().find(|d| d.id == id)).map(|d| d.called());
        let what = h.look.trim_end_matches('.').to_lowercase();
        let way = compass(self.embark, h.lair);
        let text = match &who {
            Some(n) => format!("{} has found out {}: {}, called {}, that lairs to the {}. It cannot bear {}!", n, h.nickname, what, h.name, way, h.bane.name()),
            None => format!("Now we know {}: {}, called {}, that lairs to the {}. It cannot bear {}!", h.nickname, what, h.name, way, h.bane.name()),
        };
        if let Some(h) = self.horrors.haunt.as_mut() {
            h.stage = Stage::Known;
            h.since = self.tick;
        }
        self.count("haunt.known", 1);
        self.msg(&text, (230, 200, 120));
        self.spotlight("The thing in the night", &text, Some(h.lair), (230, 200, 120));
        let by_whom = who.unwrap_or_else(|| "The dwarves".to_string());
        self.legend(format!("{} of {} learned that {}, {}, could not bear {}.", by_whom, fort, cap(&h.nickname), h.name, h.bane.name()));
        self.arm_against_haunt(true);
        // With no overseer to send the hunters, the player may: it shows
        // itself at its lair, where a squad can be ordered to kill it.
        if self.overseer.is_none() {
            self.haunt_creature(false);
        }
    }

    /// The fort readies the bane for the hunters: gold from the stocks, wood
    /// for torches, water from a running stream, a copy of an engraving it
    /// has seen. Tried when the bane is found (`loud`: told either way) and
    /// each day after until it works (told only then).
    pub(super) fn arm_against_haunt(&mut self, loud: bool) {
        let Some(bane) = self.horrors.haunt.as_ref().filter(|h| !h.armed).map(|h| h.bane) else { return };
        let free = |it: &Item| it.carried_by.is_none() && it.claimed_by.is_none() && it.container.is_none();
        let ready = match bane {
            Bane::Gold => {
                let id = self.items.values().find(|it| free(it) && matches!(it.mat, Mat::Gold | Mat::NativeGold)).map(|it| it.id);
                id.map(|id| self.items.remove(&id)).is_some()
            }
            Bane::Fire => {
                let id = self.items.values().find(|it| free(it) && it.kind == ItemKind::Log).map(|it| it.id);
                id.map(|id| self.items.remove(&id)).is_some()
            }
            // A river or brook, not a pond or a puddle.
            Bane::RunningWater => self.world.river_beds.iter().any(|&(p, _)| self.world.get(p).map_or(false, |t| t.water > 0 && !t.frozen)),
            Bane::Engraving => self.world.tiles.iter().any(|t| t.smooth >= 2 && t.revealed),
        };
        let nick = self.horrors.haunt.as_ref().map(|h| h.nickname.clone()).unwrap_or_default();
        if ready {
            if let Some(h) = self.horrors.haunt.as_mut() {
                h.armed = true;
            }
            self.msg(&format!("Against {}, the fort {}.", nick, bane.armed_text()), (230, 200, 120));
        } else if loud {
            self.msg(&format!("The fort cannot arm itself against {}: {}.", nick, bane.lacking()), (240, 170, 110));
        }
    }

    /// It, on the map at its lair (or already there): hunting the hunters
    /// if `coming` is false, marching on the fort if it's true.
    fn haunt_creature(&mut self, coming: bool) -> Option<u32> {
        let h = self.horrors.haunt.clone()?;
        if let Some(id) = h.creature.filter(|id| self.creatures.get(id).map_or(false, |c| c.alive)) {
            if coming {
                if let Some(c) = self.creatures.get_mut(&id) {
                    c.siege = true;
                }
            }
            return Some(id);
        }
        let at = if self.world.walkable(h.lair) { h.lair } else { self.edge_entry()? };
        let id = self.spawn_creature(Species::CaveHorror, at, u64::MAX);
        let mut rng = self.horror_rng(0, 0x4A_9F);
        if let Some(c) = self.creatures.get_mut(&id) {
            c.siege = coming;
            c.label = Some(format!("{}, {}", h.name, h.nickname));
            c.desc = Some(h.look.clone());
            c.body = Body::new(Species::CaveHorror.info().scale);
        }
        self.gift_powers(id, h.powers.clone(), &mut rng);
        if let Some(h) = self.horrors.haunt.as_mut() {
            h.creature = Some(id);
        }
        Some(id)
    }

    /// Nobody went to it, or the hunt lost it: it comes to the fort.
    pub(super) fn haunt_comes(&mut self) {
        let Some(h) = self.horrors.haunt.clone() else { return };
        if h.stage >= Stage::Coming {
            return;
        }
        let Some(id) = self.haunt_creature(true) else { return };
        if let Some(h) = self.horrors.haunt.as_mut() {
            h.stage = Stage::Coming;
            h.since = self.tick;
        }
        let at = self.creatures.get(&id).map(|c| c.pos);
        let text = format!("{} comes out of the {} for the fort!", cap(&h.nickname), compass(self.embark, h.lair));
        self.count("haunt.comes", 1);
        self.msg(&text, (255, 110, 160));
        self.spotlight("It comes", &text, at, (255, 110, 160));
        self.pause_request = true;
    }

    /// The overseer, every ten ticks (overseer_hooks.rs): hears the tales,
    /// and once the thing is known, hunts it, or doesn't, by temper.
    pub(super) fn ov_haunt(&mut self, o: &mut Overseer) {
        let Some(h) = self.horrors.haunt.as_ref() else { return };
        if h.stage == Stage::Slain {
            return;
        }
        let tick = self.tick;
        if h.signs.len() > h.heard {
            let nick = h.nickname.clone();
            if let Some(h) = self.horrors.haunt.as_mut() {
                h.heard = h.signs.len();
            }
            if o.style == Style::Reckless {
                Self::note_ov(o, tick, format!("Paid no mind to tales of {}.", nick));
            } else if o.has(Trait::Cautious) {
                // Nobody works outdoors for a day.
                self.hunts.outdoor_ban_until = self.hunts.outdoor_ban_until.max(tick + DAY);
                self.warn("haunt_indoors", &format!("For fear of {}, nobody works outdoors today.", nick), (200, 170, 230));
                Self::note_ov(o, tick, format!("Kept everyone indoors for fear of {}.", nick));
            } else if o.has(Trait::Bold) {
                Self::note_ov(o, tick, format!("Laughed at the tales of {}.", nick));
            } else if o.has(Trait::Pious) {
                Self::note_ov(o, tick, format!("Called for prayers against {}.", nick));
            } else {
                Self::note_ov(o, tick, format!("Heard the tales of {}.", nick));
            }
        }
        let Some(h) = self.horrors.haunt.clone() else { return };
        match h.stage {
            Stage::Known if self.wants_haunt_hunt(o, &h) => self.hunt_haunt(o),
            Stage::Hunted => self.keep_haunt_hunt(o, &h),
            _ => {}
        }
    }

    /// Whether to go out after it now: the reckless never do, the cautious
    /// wait behind the walls; the bold go at once with two fit soldiers;
    /// anyone else once the bane is ready and three can go (four without it).
    fn wants_haunt_hunt(&self, o: &Overseer, h: &Haunt) -> bool {
        if o.style == Style::Reckless || (o.has(Trait::Cautious) && !o.has(Trait::Bold)) {
            return false;
        }
        if self.alert || self.incoming.is_some() || self.squads.iter().any(|s| matches!(s.order, SquadOrder::Kill(_))) {
            return false;
        }
        let Some((_, fit, _, _)) = self.best_hunters() else { return false };
        if o.has(Trait::Bold) || o.style == Style::Warlord {
            fit.len() >= 2
        } else {
            fit.len() >= if h.armed { 3 } else { 4 }
        }
    }

    /// Send the best squad to its lair.
    fn hunt_haunt(&mut self, o: &mut Overseer) {
        let Some((s, ids, _, _)) = self.best_hunters() else { return };
        let Some(id) = self.haunt_creature(false) else { return };
        let Some(h) = self.horrors.haunt.clone() else { return };
        let squad = self.squads[s].name.clone();
        self.squads[s].order = SquadOrder::Kill(id);
        self.call_out(&ids);
        if let Some(hh) = self.horrors.haunt.as_mut() {
            hh.stage = Stage::Hunted;
            hh.since = self.tick;
            hh.squad = Some(self.squads[s].id);
        }
        let with = if h.armed { format!(", armed with {}", h.bane.name()) } else { String::new() };
        let text = format!("{} march out to the {} to hunt {}{}!", squad, compass(self.embark, h.lair), h.nickname, with);
        self.count("haunt.hunts", 1);
        self.msg(&text, (255, 200, 90));
        self.spotlight("The hunt for the thing in the night", &text, Some(h.lair), (255, 200, 90));
        Self::note_ov(o, self.tick, format!("Sent {} to hunt {}.", squad, h.nickname));
    }

    /// The hunt goes on: the squad keeps its order (the alert would station
    /// it), and comes home if it can't go on.
    fn keep_haunt_hunt(&mut self, o: &mut Overseer, h: &Haunt) {
        let (Some(sid), Some(cid)) = (h.squad, h.creature) else { return };
        let Some(s) = self.squads.iter().position(|q| q.id == sid) else { return };
        if !self.creatures.get(&cid).map_or(false, |c| c.alive) {
            return;
        }
        // Its own fit members, whether or not another squad would now be
        // the better pick.
        let (fit, _) = self.hunters(s);
        if fit.is_empty() {
            let squad = self.squads[s].name.clone();
            self.squads[s].order = SquadOrder::Train;
            self.msg(&format!("{} come home from the hunt for {}, hurt.", squad, h.nickname), (200, 200, 160));
            Self::note_ov(o, self.tick, format!("Called {} home from the hunt for {}.", squad, h.nickname));
            self.haunt_comes();
            return;
        }
        if self.squads[s].order != SquadOrder::Kill(cid) {
            self.squads[s].order = SquadOrder::Kill(cid);
        }
    }

    /// A dwarf's blow has landed on it (`struck_horror`): armed with its bane
    /// (or swinging gold at it), it hurts far worse, and the first time
    /// gives it away.
    pub(super) fn bane_blow(&mut self, id: u32, d: &Dwarf, body: &mut Body, hurt: bool) {
        let Some(h) = self.horrors.haunt.clone() else { return };
        if h.creature != Some(id) || !hurt || body.dead {
            return;
        }
        let gold = d.held.and_then(|w| self.items.get(&w)).map_or(false, |it| matches!(it.mat, Mat::Gold | Mat::NativeGold));
        let works = (h.armed && d.soldier) || (h.bane == Bane::Gold && gold);
        if !works {
            return;
        }
        let part = *self.rng.pick(&crate::body::PARTS);
        let extra = self.knob("haunt.bane", 6.0) as i32;
        if !body.lost[part.idx()] {
            body.wound(part, extra, false);
        } else {
            body.wound(crate::body::Part::UpperBody, extra, false);
        }
        self.count("haunt.bane_blows", 1);
        self.felt_bane(&h);
    }

    /// Told once: it shrieks at its bane; if nobody knew, now they do.
    fn felt_bane(&mut self, h: &Haunt) {
        if h.burnt {
            return;
        }
        if let Some(hh) = self.horrors.haunt.as_mut() {
            hh.burnt = true;
        }
        self.msg(&format!("{} shrieks as {} bites!", cap(&h.nickname), h.bane.bites()), (255, 220, 120));
        if h.stage < Stage::Known {
            self.haunt_found_out(None);
        }
    }

    /// Every horror tick: fire, running water or an engraving beside it
    /// sears it, whoever brought it there.
    pub(super) fn bane_tick(&mut self) {
        let Some(h) = self.horrors.haunt.clone() else { return };
        let Some(id) = h.creature else { return };
        let Some(c) = self.creatures.get(&id).filter(|c| c.alive) else { return };
        let p = c.pos;
        let around = || DIRS8.iter().map(move |&(dx, dy)| p.offset(dx, dy, 0)).chain([p]);
        let w = &self.world;
        let near = match h.bane {
            Bane::Fire => around().any(|q| w.get(q).map_or(false, |t| t.fire > 0)),
            Bane::RunningWater => around().any(|q| w.get(q).map_or(false, |t| t.water > 0 && !t.frozen)),
            Bane::Engraving => around().any(|q| w.get(q).map_or(false, |t| t.smooth >= 2)),
            Bane::Gold => false,
        };
        if !near {
            return;
        }
        let dmg = self.knob("haunt.bane_sear", 3.0) as i32;
        let dead = match self.creatures.get_mut(&id) {
            Some(c) => {
                c.body.wound(crate::body::Part::UpperBody, dmg, false);
                c.body.dead
            }
            None => return,
        };
        self.felt_bane(&h);
        if dead {
            if let Some(c) = self.creatures.remove(&id) {
                self.creature_died(c);
            }
        }
    }

    /// While the hunters carry its bane, its wounds don't close.
    pub(super) fn bane_stops_regrowth(&self, id: u32) -> bool {
        self.horrors.haunt.as_ref().map_or(false, |h| h.creature == Some(id) && (h.armed || h.burnt))
    }

    /// It has died (`horror_died`): the tale ends, and the legends keep it.
    pub(super) fn haunt_died(&mut self, c: &Creature) {
        let Some(h) = self.horrors.haunt.clone() else { return };
        if h.creature != Some(c.id) || h.stage == Stage::Slain {
            return;
        }
        if let Some(hh) = self.horrors.haunt.as_mut() {
            hh.stage = Stage::Slain;
            hh.since = self.tick;
        }
        let fort = self.fortress_name.clone();
        let text = format!("{} is dead! {}, that haunted {}, will drink no more.", cap(&h.nickname), h.name, fort);
        self.count("haunt.slain", 1);
        self.msg(&text, (160, 255, 160));
        self.spotlight("The thing in the night is slain", &text, Some(c.pos), (160, 255, 160));
        let how = if h.armed || h.burnt { format!(" with {}", h.bane.name()) } else { String::new() };
        self.legend(format!("The dwarves of {} slew {}, {}{}.", fort, h.name, h.nickname, how));
        let tick = self.tick;
        for d in self.dwarves.iter_mut().filter(|d| d.alive) {
            d.think(&format!("was glad {} is dead", h.nickname), -10, tick);
        }
    }

    /// Lines for the look panel, on it.
    pub(super) fn haunt_look(&self, c: &Creature) -> Vec<String> {
        let Some(h) = self.horrors.haunt.as_ref().filter(|h| h.creature == Some(c.id)) else { return Vec::new() };
        let mut v = vec![format!("The thing the dwarves call {}.", h.nickname)];
        if h.stage >= Stage::Known || h.burnt {
            v.push(format!("It cannot bear {}.", h.bane.name()));
        }
        v
    }

    /// Lines for the fortress status: what is known of it so far.
    pub fn haunt_lines(&self) -> Vec<String> {
        let Some(h) = self.horrors.haunt.as_ref() else { return Vec::new() };
        let nick = cap(&h.nickname);
        let mut v = Vec::new();
        match h.stage {
            Stage::Signs | Stage::Investigating => {
                v.push(format!("Something haunts the land about {}. The dwarves call it {}.", self.fortress_name, h.nickname));
                if let Some(s) = h.signs.last() {
                    v.push(format!("Lately: {}", s));
                }
                if let Some(d) = h.investigator.and_then(|id| self.dwarves.iter().find(|d| d.id == id)) {
                    v.push(format!("{} is looking into it ({} of {} clues).", d.called(), h.clues, CLUES));
                }
            }
            Stage::Known => {
                v.push(format!("{} is {}, lairing to the {}.", nick, h.look.trim_end_matches('.').to_lowercase(), compass(self.embark, h.lair)));
                v.push(format!("It cannot bear {}{}.", h.bane.name(), if h.armed { "; the hunters are armed with it" } else { "" }));
            }
            Stage::Hunted => v.push(format!("The squad hunts {} to the {}.", h.nickname, compass(self.embark, h.lair))),
            Stage::Coming => v.push(format!("{} is coming! It cannot bear {}.", nick, h.bane.name())),
            Stage::Slain => v.push(format!("{} ({}) is slain.", nick, h.name)),
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Mat;

    /// A fort whose tale has begun, a year in.
    fn haunted(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g.tick = YEAR;
        g.knobs.set.insert("haunt.odds".into(), 1.0);
        g.knobs.set.insert("haunt.chance".into(), 1.0);
        g.haunt_daily();
        assert!(g.horrors.haunt.is_some(), "it never came");
        g
    }

    /// Days go by, with only the tale's daily round.
    fn days(g: &mut Game, n: u64) {
        for _ in 0..n {
            g.tick += DAY;
            g.haunt_daily();
        }
    }

    #[test]
    fn the_same_world_and_site_send_the_same_thing_and_it_keeps() {
        let a = Game::new(81).make_haunt();
        let b = Game::new(81).make_haunt();
        assert_eq!(a, b);
        assert!(a.nickname.starts_with("the "));
        assert!(!a.powers.is_empty());
        let back: Haunt = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        assert_eq!(back, a);
        // Not before the first year is out.
        let mut g = Game::new(81);
        g.knobs.set.insert("haunt.odds".into(), 1.0);
        g.knobs.set.insert("haunt.chance".into(), 1.0);
        g.tick = YEAR / 2;
        g.haunt_daily();
        assert!(g.horrors.haunt.is_none());
        // Different sites, different things.
        let others: Vec<String> = [5u64, 9, 13, 21].iter().map(|&s| Game::new(s).make_haunt().nickname).collect();
        assert!(others.iter().any(|n| *n != a.nickname), "{:?}", others);
    }

    #[test]
    fn signs_lead_to_an_investigation_that_finds_its_bane() {
        let mut g = haunted(81);
        assert!(g.spotlights.iter().any(|s| s.title == "Something in the night"));
        let sheriff = g.dwarves.iter().find(|d| d.alive && !d.is_child()).unwrap().id;
        g.positions.push((Position::Sheriff, sheriff));
        days(&mut g, 30);
        let h = g.horrors.haunt.clone().unwrap();
        assert!(h.signs.len() >= 2, "{:?}", h.signs);
        assert_eq!(h.investigator, Some(sheriff));
        assert!(h.stage >= Stage::Known, "still at {:?} with {} clues", h.stage, h.clues);
        assert!(g.log.iter().any(|l| l.text.contains("has begun to look into")));
        assert!(g.log.iter().any(|l| l.text.contains(&format!("cannot bear {}", h.bane.name()))));
        assert!(g.overworld.events.iter().any(|e| e.text.contains(&format!("could not bear {}", h.bane.name()))));
        assert!(g.spotlights.iter().any(|s| s.title == "The thing in the night"));
        assert!(g.haunt_lines().iter().any(|l| l.contains(h.bane.name())));
        // Nobody goes to it: it comes.
        days(&mut g, 20);
        let h = g.horrors.haunt.clone().unwrap();
        assert_eq!(h.stage, Stage::Coming);
        let c = &g.creatures[&h.creature.unwrap()];
        assert!(c.siege && c.label.as_deref().unwrap().contains(&h.nickname));
        assert_eq!(g.horrors.gifts[&c.id].powers, h.powers);
        assert!(g.haunt_look(c).iter().any(|l| l.contains(h.bane.name())));
    }

    #[test]
    fn its_signs_drain_livestock_and_take_haulers() {
        let mut g = haunted(81);
        let n0 = g.creatures.values().filter(|c| c.tame).count();
        // Everyone and everything out in the open.
        let e = g.embark;
        for d in g.dwarves.iter_mut() {
            d.pos = e;
        }
        for c in g.creatures.values_mut() {
            c.pos = e;
        }
        for k in 0..60u64 {
            let mut r = Rng::new(k);
            g.haunt_sign(&mut r);
        }
        let h = g.horrors.haunt.clone().unwrap();
        assert!(h.signs.iter().any(|s| s.contains("drained of blood")), "{:?}", h.signs);
        assert!(h.signs.iter().any(|s| s.contains("went missing")), "{:?}", h.signs);
        assert!(g.creatures.values().filter(|c| c.tame).count() < n0);
        assert!(g.dwarves.iter().any(|d| d.thoughts.iter().any(|t| t.text.contains("carried off"))));
    }

    #[test]
    fn its_bane_bites_and_stops_its_wounds_closing() {
        let mut g = haunted(81);
        g.horrors.haunt.as_mut().unwrap().powers = vec![Power::Regrowth];
        let id = g.haunt_creature(false).unwrap();
        let mut d = g.dwarves[0].clone();
        d.soldier = true;
        let body0 = g.creatures[&id].body.clone();
        // Unarmed with it: no more than the blow.
        let mut b = body0.clone();
        g.bane_blow(id, &d, &mut b, true);
        assert_eq!(b.hp, body0.hp);
        // Armed: it bites, and the fort learns it if it didn't know.
        g.horrors.haunt.as_mut().unwrap().armed = true;
        let mut b = body0.clone();
        g.bane_blow(id, &d, &mut b, true);
        assert!(b.hp.iter().sum::<i32>() < body0.hp.iter().sum::<i32>());
        assert!(g.log.iter().any(|l| l.text.contains("shrieks")));
        assert!(g.horrors.haunt.as_ref().unwrap().stage >= Stage::Known);
        // Its wounds no longer knit.
        g.creatures.get_mut(&id).unwrap().body.wound(crate::body::Part::LeftLeg, 6, false);
        let before: i32 = g.creatures[&id].body.hp.iter().sum();
        g.horrors_tick();
        assert_eq!(g.creatures[&id].body.hp.iter().sum::<i32>(), before);
        // A gold blade bites even unarmed, if gold is its bane.
        let mut g = haunted(81);
        g.horrors.haunt.as_mut().unwrap().bane = Bane::Gold;
        let id = g.haunt_creature(false).unwrap();
        let mut d = g.dwarves[0].clone();
        let axe = g.spawn_item(ItemKind::Axe, Mat::Gold, d.pos);
        d.held = Some(axe);
        let body0 = g.creatures[&id].body.clone();
        let mut b = body0.clone();
        g.bane_blow(id, &d, &mut b, true);
        assert!(b.hp.iter().sum::<i32>() < body0.hp.iter().sum::<i32>());
        // And fire at its feet sears one whose bane is fire.
        let mut g = haunted(81);
        g.horrors.haunt.as_mut().unwrap().bane = Bane::Fire;
        let id = g.haunt_creature(false).unwrap();
        let p = g.creatures[&id].pos;
        let hp0: i32 = g.creatures[&id].body.hp.iter().sum();
        g.world.t_mut(p.offset(1, 0, 0)).fire = 10;
        g.bane_tick();
        assert!(g.creatures[&id].body.hp.iter().sum::<i32>() < hp0);
    }

    /// An overseer fort with a known horror and a squad of `n` armed soldiers.
    fn known(seed: u64, traits: Vec<Trait>, n: usize) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g.set_overseer(Some(Style::Steady));
        g.overseer.as_mut().unwrap().traits = traits;
        let ids: Vec<u32> = g.dwarves.iter().filter(|d| d.alive && !d.is_child()).take(n).map(|d| d.id).collect();
        let s = g.new_squad();
        for &id in &ids {
            g.set_squad_member(s, id, true);
            let at = g.dwarves.iter().find(|d| d.id == id).unwrap().pos;
            let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, at);
            g.items.get_mut(&axe).unwrap().carried_by = Some(id);
            g.dwarves.iter_mut().find(|d| d.id == id).unwrap().held = Some(axe);
        }
        g.tick = YEAR;
        // (The beasts that set it off are gone.)
        g.alert = false;
        g.knobs.set.insert("haunt.odds".into(), 1.0);
        g.knobs.set.insert("haunt.chance".into(), 1.0);
        g.haunt_daily();
        g.horrors.haunt.as_mut().unwrap().signs.push("tracks".into());
        g
    }

    fn overseer_round(g: &mut Game) {
        let mut o = g.overseer.take().unwrap();
        g.ov_haunt(&mut o);
        g.overseer = Some(o);
    }

    #[test]
    fn the_overseer_hunts_it_or_hides_from_it_by_temper() {
        // Cautious: everyone indoors at the first tales, and no hunt.
        let mut g = known(81, vec![Trait::Cautious], 3);
        overseer_round(&mut g);
        assert!(g.outdoors_banned());
        assert!(g.overseer.as_ref().unwrap().notes.iter().any(|l| l.1.contains("indoors")));
        g.haunt_found_out(None);
        overseer_round(&mut g);
        assert_eq!(g.horrors.haunt.as_ref().unwrap().stage, Stage::Known);
        // Bold: out after it at once, two being enough.
        let mut g = known(81, vec![Trait::Bold], 2);
        g.haunt_found_out(None);
        overseer_round(&mut g);
        let h = g.horrors.haunt.clone().unwrap();
        assert_eq!(h.stage, Stage::Hunted);
        let cid = h.creature.unwrap();
        assert!(!g.creatures[&cid].siege);
        assert!(g.squads.iter().any(|s| s.order == SquadOrder::Kill(cid)));
        assert!(g.spotlights.iter().any(|s| s.title.starts_with("The hunt for")));
        // The alert can't call them off.
        let s = g.squads.iter().position(|s| s.order == SquadOrder::Kill(cid)).unwrap();
        g.squads[s].order = SquadOrder::Station;
        overseer_round(&mut g);
        assert_eq!(g.squads[s].order, SquadOrder::Kill(cid));
        // Steady, with no bane to hand and only three: waits.
        let mut g = known(81, Vec::new(), 3);
        g.haunt_found_out(None);
        g.horrors.haunt.as_mut().unwrap().armed = false;
        overseer_round(&mut g);
        assert_eq!(g.horrors.haunt.as_ref().unwrap().stage, Stage::Known);
        // Armed, three will do.
        g.horrors.haunt.as_mut().unwrap().armed = true;
        overseer_round(&mut g);
        assert_eq!(g.horrors.haunt.as_ref().unwrap().stage, Stage::Hunted);
    }

    #[test]
    fn a_second_squad_does_not_call_the_hunters_home() {
        // Two go out; then a bigger squad is raised at home.
        let mut g = known(81, vec![Trait::Bold], 2);
        g.haunt_found_out(None);
        overseer_round(&mut g);
        let h = g.horrors.haunt.clone().unwrap();
        assert_eq!(h.stage, Stage::Hunted);
        let cid = h.creature.unwrap();
        let hunting = g.squads.iter().position(|s| s.order == SquadOrder::Kill(cid)).unwrap();
        let busy: Vec<u32> = g.squads[hunting].members.clone();
        let more: Vec<u32> = g.dwarves.iter().filter(|d| d.alive && !d.is_child() && !busy.contains(&d.id)).take(3).map(|d| d.id).collect();
        assert_eq!(more.len(), 3);
        let s2 = g.new_squad();
        for &id in &more {
            g.set_squad_member(s2, id, true);
            let at = g.dwarves.iter().find(|d| d.id == id).unwrap().pos;
            let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, at);
            g.items.get_mut(&axe).unwrap().carried_by = Some(id);
            g.dwarves.iter_mut().find(|d| d.id == id).unwrap().held = Some(axe);
        }
        assert_ne!(g.best_hunters().unwrap().0, hunting, "the new squad should outrank the hunters");
        overseer_round(&mut g);
        assert_eq!(g.horrors.haunt.as_ref().unwrap().stage, Stage::Hunted);
        assert_eq!(g.squads[hunting].order, SquadOrder::Kill(cid));
        assert!(!g.creatures[&cid].siege);
    }

    #[test]
    fn the_fort_arms_itself_when_the_bane_comes_to_hand() {
        let mut g = haunted(81);
        g.horrors.haunt.as_mut().unwrap().bane = Bane::Gold;
        g.items.retain(|_, it| !matches!(it.mat, Mat::Gold | Mat::NativeGold));
        g.haunt_found_out(None);
        assert!(!g.horrors.haunt.as_ref().unwrap().armed);
        let at = g.dwarves[0].pos;
        g.spawn_item(ItemKind::Bar, Mat::Gold, at);
        days(&mut g, 1);
        assert!(g.horrors.haunt.as_ref().unwrap().armed, "the gold that came in was never used");
        // A still pond is not running water.
        let mut g = haunted(81);
        g.horrors.haunt.as_mut().unwrap().bane = Bane::RunningWater;
        g.world.river_beds.clear();
        g.world.t_mut(g.embark).water = 7;
        g.haunt_found_out(None);
        assert!(!g.horrors.haunt.as_ref().unwrap().armed);
    }

    #[test]
    fn in_a_players_fort_it_shows_itself_at_its_lair_once_known() {
        let mut g = haunted(81);
        assert!(g.overseer.is_none());
        g.haunt_found_out(None);
        let h = g.horrors.haunt.clone().unwrap();
        let id = h.creature.expect("not on the map for the player to hunt");
        let c = &g.creatures[&id];
        assert!(c.alive && !c.siege);
        assert_eq!(h.stage, Stage::Known);
        // An overseer's fort waits for the overseer to send the hunt.
        let mut g = known(81, vec![Trait::Cautious], 3);
        g.haunt_found_out(None);
        assert!(g.horrors.haunt.as_ref().unwrap().creature.is_none());
    }

    #[test]
    fn a_fire_breather_never_fears_fire() {
        let mut g = Game::new(81);
        for seed in 0..2000 {
            g.overworld.seed = seed;
            let h = g.make_haunt();
            assert!(!(h.bane == Bane::Fire && h.powers.contains(&Power::Fire)), "seed {}", seed);
        }
    }

    #[test]
    fn a_hunt_armed_with_its_bane_goes_out_and_fights_it() {
        let mut g = known(81, vec![Trait::Bold], 3);
        g.haunt_found_out(None);
        g.horrors.haunt.as_mut().unwrap().armed = true;
        overseer_round(&mut g);
        let cid = g.horrors.haunt.as_ref().unwrap().creature.expect("no hunt");
        for _ in 0..8000 {
            g.step();
            let blows = g.annals.counters.get("haunt.bane_blows").copied().unwrap_or(0);
            if blows > 0 || !g.creatures.contains_key(&cid) {
                break;
            }
        }
        let blows = g.annals.counters.get("haunt.bane_blows").copied().unwrap_or(0);
        assert!(blows > 0 || g.horrors.haunt.as_ref().unwrap().stage == Stage::Slain, "the hunters never struck it with its bane");
    }

    #[test]
    fn when_it_dies_the_tale_ends_in_the_legends() {
        let mut g = haunted(81);
        g.haunt_comes();
        let id = g.horrors.haunt.as_ref().unwrap().creature.unwrap();
        let mut c = g.creatures.remove(&id).unwrap();
        c.body.dead = true;
        g.creature_died(c);
        let h = g.horrors.haunt.clone().unwrap();
        assert_eq!(h.stage, Stage::Slain);
        assert!(g.overworld.events.iter().any(|e| e.text.contains(&format!("slew {}, {}", h.name, h.nickname))));
        assert!(g.spotlights.iter().any(|s| s.title == "The thing in the night is slain"));
        assert!(g.haunt_lines()[0].contains("slain"));
    }
}
