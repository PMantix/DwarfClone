//! What the fortress is short of, as the announcements tell it ("…waiting
//! to be built: it needs a mechanism", "Your dwarves have nothing to drink!"),
//! kept as plain facts so the overseer can notice and see to them. How soon
//! it notices, and what it bothers with, depends on its style and on
//! whether it has a manager.

use super::overseer::{Overseer, RoomUse, Style};
use super::*;
use crate::buildings::{Input, Recipe};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lack {
    Stone,
    Wood,
    Blocks,
    Mechanism,
    Cage,
    Bucket,
    Ore,
    Fuel,
    Bars,
    Drink,
    Food,
    Brewable,
    Seeds,
    Thread,
    Cloth,
    Leather,
    Pick,
    Axe,
    Manager,
    /// A work order no workshop can do: this kind is missing.
    Workshop(BuildingKind),
    /// A dwarf in a strange mood needs a workshop of this kind.
    MoodShop(BuildingKind),
    Gem,
    Bone,
    /// Soldiers with nothing in the stores for this piece of their kit.
    Kit(super::readiness::KitPart),
    /// Someone is bleeding or senseless and no dwarf can tend them.
    Doctor,
}

impl Lack {
    /// What a building part or a construction asks for, from its words.
    pub fn from_desc(desc: &str) -> Option<Lack> {
        let has = |w: &str| desc.contains(w);
        Some(if has("mechanism") {
            Lack::Mechanism
        } else if has("cage") {
            Lack::Cage
        } else if has("bucket") {
            Lack::Bucket
        } else if has("block") {
            Lack::Blocks
        } else if has("boulder") || has("stone") || has("rock") {
            Lack::Stone
        } else if has("log") || has("wood") {
            Lack::Wood
        } else if has("bar") || has("metal") {
            Lack::Bars
        } else {
            return None;
        })
    }

    pub fn from_input(i: Input) -> Option<Lack> {
        Some(match i {
            Input::Log => Lack::Wood,
            Input::Stone | Input::Flux => Lack::Stone,
            Input::Brewable | Input::Grain => Lack::Brewable,
            Input::Fiber => Lack::Thread,
            Input::Ore => Lack::Ore,
            Input::Fuel => Lack::Fuel,
            Input::Bar | Input::IronBar => Lack::Bars,
            Input::Thread => Lack::Thread,
            Input::Cloth => Lack::Cloth,
            Input::Hide | Input::Leather => Lack::Leather,
            _ => return None,
        })
    }

    /// How soon it must be seen to: 0 can't wait (thirst, hunger, the
    /// wounded, a fey mood), 1 defence and tools, 2 the fortress's
    /// materials, 3 comforts.
    pub fn urgency(self) -> u8 {
        match self {
            Lack::Drink | Lack::Food | Lack::Doctor | Lack::MoodShop(_) => 0,
            Lack::Kit(_) | Lack::Mechanism | Lack::Cage | Lack::Pick | Lack::Axe | Lack::Manager | Lack::Seeds | Lack::Brewable => 1,
            Lack::Stone | Lack::Wood | Lack::Blocks | Lack::Workshop(_) | Lack::Ore | Lack::Fuel | Lack::Bars | Lack::Bucket => 2,
            Lack::Thread | Lack::Cloth | Lack::Leather | Lack::Gem | Lack::Bone => 3,
        }
    }

    /// What a caravan brings that makes it good (see `trade::caravan_goods`).
    pub fn buyable(self) -> Option<ItemKind> {
        Some(match self {
            Lack::Wood => ItemKind::Log,
            Lack::Bars => ItemKind::Bar,
            Lack::Pick => ItemKind::Pick,
            Lack::Axe => ItemKind::Axe,
            Lack::Drink => ItemKind::Drink,
            Lack::Food => ItemKind::Meal,
            Lack::Seeds => ItemKind::Seed,
            Lack::Fuel => ItemKind::Charcoal,
            Lack::Leather => ItemKind::Leather,
            Lack::Gem => ItemKind::Gem,
            _ => return None,
        })
    }

    /// The job that makes it, if a workshop makes it (kit: `kit_order`).
    fn recipe(self) -> Option<Recipe> {
        Some(match self {
            Lack::Blocks => Recipe::Blocks,
            Lack::Mechanism => Recipe::Mechanism,
            Lack::Cage => Recipe::WoodCage,
            Lack::Bucket => Recipe::Bucket,
            Lack::Fuel => Recipe::Charcoal,
            Lack::Bars => Recipe::SmeltOre,
            Lack::Drink => Recipe::Brew,
            Lack::Thread => Recipe::SpinThread,
            Lack::Cloth => Recipe::Weave,
            Lack::Leather => Recipe::Tan,
            Lack::Pick => Recipe::ForgePick,
            Lack::Axe => Recipe::ForgeAxe,
            _ => return None,
        })
    }

    /// A recipe's input as something the fortress can be short of (what
    /// is gathered, hunted or a by-product needs nothing first).
    fn input_need(i: Input) -> Option<Lack> {
        match i {
            Input::Hide => Some(Lack::Workshop(BuildingKind::Butcher)),
            Input::Edible => Some(Lack::Food),
            Input::RoughGem => Some(Lack::Gem),
            Input::Bone => Some(Lack::Bone),
            Input::Fiber | Input::Carcass | Input::Ash | Input::Tallow => None,
            _ => Lack::from_input(i),
        }
    }

    pub fn what(self) -> String {
        match self {
            Lack::Stone => "stone".into(),
            Lack::Wood => "wood".into(),
            Lack::Blocks => "stone blocks".into(),
            Lack::Mechanism => "mechanisms".into(),
            Lack::Cage => "cages".into(),
            Lack::Bucket => "buckets".into(),
            Lack::Ore => "ore".into(),
            Lack::Fuel => "fuel".into(),
            Lack::Bars => "metal bars".into(),
            Lack::Drink => "drink".into(),
            Lack::Food => "food".into(),
            Lack::Brewable => "plants to brew".into(),
            Lack::Seeds => "seeds".into(),
            Lack::Thread => "thread".into(),
            Lack::Cloth => "cloth".into(),
            Lack::Leather => "leather".into(),
            Lack::Pick => "picks".into(),
            Lack::Axe => "axes".into(),
            Lack::Manager => "a manager".into(),
            Lack::Workshop(k) => format!("a {}", k.name().to_lowercase()),
            Lack::MoodShop(k) => format!("a {} for a dwarf in a fey mood", k.name().to_lowercase()),
            Lack::Gem => "gems".into(),
            Lack::Bone => "bone".into(),
            Lack::Kit(p) => format!("{} for the soldiers", p.plural()),
            Lack::Doctor => "a doctor for the wounded".into(),
        }
    }
}

/// How long a trouble is remembered after it was last announced.
const MEMORY: u64 = DAY * 3;

/// A shortage and everything it waits on.
#[derive(Clone, Debug)]
pub(super) struct Chain {
    pub top: Lack,
    /// What isn't there yet, deepest first, ending with `top`.
    pub steps: Vec<Lack>,
    /// The steps with nothing left to wait on: what to work on now.
    pub roots: Vec<Lack>,
    /// A circle in the chain (each waits on the next, the last on the first).
    pub cycle: Option<Vec<Lack>>,
    /// Other troubles waiting on the same things, seen to with this one.
    pub also: Vec<Lack>,
}

impl Chain {
    /// Everything this worry is about, `top` first.
    fn tops(&self) -> Vec<Lack> {
        std::iter::once(self.top).chain(self.also.iter().copied()).collect()
    }
}

/// One of the overseer's worries: a shortage it is seeing to as a whole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Worry {
    pub lack: Lack,
    /// "first stone, then a mason's workshop" (empty: nothing to wait on).
    pub plan: String,
    /// How it stands, for the notes panel.
    pub status: String,
    /// Parked till then (a circle it can't break before then).
    pub until: u64,
    /// What it waits on a caravan for.
    pub buying: Option<ItemKind>,
    /// Its last note on this worry, and when.
    pub said: String,
    pub noted: u64,
    /// Other troubles it sees to as part of this one.
    #[serde(default)]
    pub also: Vec<Lack>,
}

impl Worry {
    /// Whether its last note on this is still to be read.
    fn heard(&self, o: &Overseer) -> bool {
        o.notes.iter().any(|n| n.0 == self.noted && n.1 == self.said)
    }
}

/// How long before it says the same of a worry again.
const RESAY: u64 = SEASON / 2;

impl Overseer {
    /// Its worries as a plan: what it's doing now, what waits on what, what
    /// it is buying or has given up on.
    pub fn plan(&self) -> Vec<String> {
        self.worries.iter().map(|w| w.status.clone()).collect()
    }
}

/// "stone, then a mason's workshop, then stone blocks".
fn then_list(l: &[Lack]) -> String {
    l.iter().map(|x| x.what()).collect::<Vec<_>>().join(", then ")
}

/// What a worry is about: "helms, mail and shields for the soldiers".
fn names(tops: &[Lack]) -> String {
    let kit: Vec<&str> = tops.iter().filter_map(|l| if let Lack::Kit(p) = l { Some(p.plural()) } else { None }).collect();
    if kit.len() == tops.len() && kit.len() > 1 {
        let n = kit.len();
        return format!("{} and {} for the soldiers", kit[..n - 1].join(", "), kit[n - 1]);
    }
    and_list(tops)
}

/// "stone and wood".
fn and_list(l: &[Lack]) -> String {
    let w: Vec<String> = l.iter().map(|x| x.what()).collect();
    match w.len() {
        0 => String::new(),
        1 => w[0].clone(),
        n => format!("{} and {}", w[..n - 1].join(", "), w[n - 1]),
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// "a mason's workshop" → "mason's workshop".
fn bare(s: &str) -> String {
    s.strip_prefix("a ").or(s.strip_prefix("an ")).unwrap_or(s).to_string()
}

impl Game {
    /// Something is short: remember it (an announcement said so).
    pub(super) fn trouble(&mut self, lack: Lack) {
        let tick = self.tick;
        match self.troubles.iter_mut().find(|t| t.0 == lack) {
            Some(t) => t.1 = tick,
            None => self.troubles.push((lack, tick)),
        }
    }

    /// How sharp an overseer is about the fortress's wants: by its style,
    /// sharper with a manager to keep track.
    fn attention(&self, o: &Overseer, lack: Lack) -> f64 {
        let base: f64 = match o.style {
            Style::Steady => 0.85,
            Style::Builder => 0.75,
            Style::Delver => 0.7,
            Style::Warlord => 0.6,
            Style::Reckless => 0.3,
        };
        let manager = if self.has_post(super::positions::Position::Manager) { 0.1 } else { 0.0 };
        use super::personality::Trait;
        let temper = if o.has(Trait::Stubborn) { 0.7 } else if o.has(Trait::Cautious) { 1.1 } else { 1.0 };
        ((base + manager) * Self::care(o, lack) * temper).min(0.98)
    }

    /// What each overseer cares about most (and least), by style and traits.
    fn care(o: &Overseer, lack: Lack) -> f64 {
        let care = match (o.style, lack) {
            (Style::Builder, Lack::Stone | Lack::Blocks | Lack::Mechanism | Lack::Workshop(_)) => 1.25,
            (Style::Delver, Lack::Ore | Lack::Stone | Lack::Pick | Lack::Fuel | Lack::Bars) => 1.25,
            (Style::Warlord, Lack::Bars | Lack::Mechanism | Lack::Cage | Lack::Ore) => 1.3,
            (Style::Warlord, Lack::Thread | Lack::Cloth | Lack::Leather) => 0.6,
            (Style::Reckless, Lack::Mechanism | Lack::Cage | Lack::Bucket) => 0.3,
            (_, Lack::Drink | Lack::Food | Lack::Doctor) => 1.4,
            // A dwarf in a fey mood who's kept waiting goes mad.
            (_, Lack::MoodShop(_)) => 1.6,
            _ => 1.0,
        };
        use super::personality::Trait;
        if o.has(Trait::Sentimental) && matches!(lack, Lack::MoodShop(_)) {
            care * 1.2
        } else {
            care
        }
    }

    /// What has to be there before `lack` can be made good, read off what
    /// the code asks for: a workshop's building material (`needs_desc`), a
    /// recipe's inputs and the workshop that does it (`recipes`,
    /// `inputs_at`: no fuel at a magma shop), a pick to quarry or mine, an
    /// axe to chop, seed to grow plants, a butcher for hides and bone.
    pub(super) fn prereqs(&self, lack: Lack) -> Vec<Lack> {
        let mut v: Vec<Lack> = Vec::new();
        match lack {
            Lack::Stone | Lack::Ore | Lack::Gem => v.push(Lack::Pick),
            Lack::Wood => v.push(Lack::Axe),
            Lack::Brewable => v.push(Lack::Seeds),
            Lack::Bone => v.push(Lack::Workshop(BuildingKind::Butcher)),
            Lack::Workshop(k) | Lack::MoodShop(k) => v.extend(Lack::from_desc(k.needs_desc())),
            _ => {}
        }
        let recipe = match lack {
            Lack::Kit(p) => self.kit_order(p).map(|r| r.0),
            l => l.recipe(),
        };
        if let Some(r) = recipe {
            let shops: Vec<BuildingKind> = BuildingKind::ALL.iter().copied().filter(|k| k.recipes().contains(&r)).collect();
            let built = shops.iter().copied().find(|&k| self.buildings.values().any(|b| b.built && b.kind == k));
            if let Some(shop) = built.or(shops.first().copied()) {
                for i in r.inputs_at(shop) {
                    if let Some(l) = Lack::input_need(i) {
                        if l != lack && !v.contains(&l) {
                            v.push(l);
                        }
                    }
                }
                v.push(Lack::Workshop(shop));
            }
        }
        v
    }

    /// Whether the fortress has it now (so nothing waits on it).
    pub(super) fn have(&self, lack: Lack) -> bool {
        let free = |f: &dyn Fn(&Item) -> bool| self.items.values().any(|it| it.free() && f(it));
        let kind = |k: ItemKind| free(&|it: &Item| it.kind == k);
        match lack {
            Lack::Stone => free(&|it| Input::Stone.accepts(it)),
            Lack::Wood => kind(ItemKind::Log),
            Lack::Blocks => kind(ItemKind::Block),
            Lack::Mechanism => kind(ItemKind::Mechanism),
            Lack::Cage => free(&|it| it.kind == ItemKind::Cage && it.label.is_none()),
            Lack::Bucket => kind(ItemKind::Bucket),
            Lack::Ore => free(&|it| Input::Ore.accepts(it)),
            Lack::Fuel => free(&|it| Input::Fuel.accepts(it)),
            Lack::Bars => kind(ItemKind::Bar),
            Lack::Drink => kind(ItemKind::Drink),
            Lack::Food => free(&|it| it.kind == ItemKind::Meal || Input::Edible.accepts(it)),
            Lack::Brewable => free(&|it| Input::Brewable.accepts(it)),
            Lack::Seeds => kind(ItemKind::Seed),
            Lack::Thread => kind(ItemKind::Thread),
            Lack::Cloth => kind(ItemKind::Cloth),
            Lack::Leather => kind(ItemKind::Leather),
            Lack::Gem => free(&|it| it.kind == ItemKind::Gem || Input::RoughGem.accepts(it)),
            Lack::Bone => kind(ItemKind::Bone),
            // A tool in a miner's hand still digs.
            Lack::Pick | Lack::Axe => {
                let k = if lack == Lack::Pick { ItemKind::Pick } else { ItemKind::Axe };
                self.items.values().any(|it| it.kind == k)
            }
            Lack::Manager => self.has_post(super::positions::Position::Manager),
            Lack::Doctor => self.has_doctor(),
            // (A magma smelter does what a smelter does.)
            Lack::Workshop(k) | Lack::MoodShop(k) => self.buildings.values().any(|b| b.built && (b.kind == k || (!k.recipes().is_empty() && b.kind.recipes() == k.recipes()))),
            Lack::Kit(_) => false,
        }
    }

    /// What `top` waits on, deepest first, ending with `top`; and the first
    /// circle found, if it waits on itself somewhere down the line.
    pub(super) fn chain(&self, top: Lack) -> Chain {
        let (mut path, mut steps, mut cycle) = (Vec::new(), Vec::new(), None);
        self.walk(top, &mut path, &mut steps, &mut cycle);
        let roots = if cycle.is_some() { Vec::new() } else { steps.iter().copied().filter(|&s| self.prereqs(s).iter().all(|&p| self.have(p))).collect() };
        Chain { top, steps, roots, cycle, also: Vec::new() }
    }

    fn walk(&self, l: Lack, path: &mut Vec<Lack>, steps: &mut Vec<Lack>, cycle: &mut Option<Vec<Lack>>) {
        path.push(l);
        for p in self.prereqs(l) {
            if steps.contains(&p) || self.have(p) {
                continue;
            }
            if let Some(i) = path.iter().position(|&x| x == p) {
                if cycle.is_none() {
                    *cycle = Some(path[i..].to_vec());
                }
                continue;
            }
            self.walk(p, path, steps, cycle);
        }
        path.pop();
        steps.push(l);
    }

    /// Today's troubles as whole chains, the most pressing first: by
    /// urgency, then by what this overseer cares about. A trouble that is
    /// only a step in a more pressing one's chain is part of that worry.
    pub(super) fn agenda(&self, o: &Overseer, open: &[Lack]) -> Vec<Chain> {
        let mut chains: Vec<Chain> = open.iter().map(|&l| self.chain(l)).collect();
        chains.sort_by(|a, b| a.top.urgency().cmp(&b.top.urgency()).then(Self::care(o, b.top).partial_cmp(&Self::care(o, a.top)).unwrap_or(std::cmp::Ordering::Equal)));
        let mut kept: Vec<Chain> = Vec::new();
        for c in chains {
            if kept.iter().any(|k| k.steps.contains(&c.top)) {
                continue;
            }
            // Waiting on nothing a kept worry isn't already waiting on: one worry.
            let waits: Vec<Lack> = c.steps.iter().copied().filter(|&l| l != c.top).collect();
            let cycle = c.cycle.clone();
            if !waits.is_empty() || cycle.is_some() {
                if let Some(k) = kept.iter_mut().find(|k| {
                    let k_waits: Vec<Lack> = k.steps.iter().copied().filter(|&l| l != k.top).collect();
                    k.cycle == cycle && waits == k_waits
                }) {
                    k.also.push(c.top);
                    continue;
                }
            }
            kept.push(c);
        }
        kept
    }

    /// Once a day: notice what's short, work out what each shortage waits
    /// on, and see to the first things first.
    pub(super) fn ov_troubles(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        self.troubles.retain(|t| tick < t.1 + MEMORY);
        o.fixes.retain(|f| tick < f.1 + DAY * 3);
        // What it waited on a caravan for has come: back to work on it.
        for w in o.worries.iter_mut() {
            if let Some(k) = w.buying {
                if self.items.values().any(|it| it.kind == k) {
                    w.until = 0;
                    w.buying = None;
                }
            }
        }
        let parked: Vec<Worry> = o.worries.iter().filter(|w| w.until > tick).cloned().collect();
        let open: Vec<Lack> = self.troubles.iter().map(|t| t.0).filter(|l| !parked.iter().any(|w| w.lack == *l || w.also.contains(l))).collect();
        let agenda = self.agenda(o, &open);
        let mut worries = parked;
        // A few worries at a time; a manager keeps track of one more.
        let cap = 3 + self.has_post(super::positions::Position::Manager) as usize;
        let mut taken = 0;
        for c in agenda {
            let known = o.worries.iter().find(|w| c.tops().iter().any(|l| w.lack == *l || w.also.contains(l))).cloned();
            if known.is_none() && !self.rng.chance(self.attention(o, c.top)) {
                continue;
            }
            if taken >= cap {
                worries.extend(known);
                continue;
            }
            taken += 1;
            let w = match &c.cycle {
                Some(cy) => self.break_cycle(o, &c.tops(), cy),
                None => self.work_chain(o, &c, known.as_ref()),
            };
            worries.push(w);
        }
        o.worries = worries;
        o.wanted = o.worries.iter().filter_map(|w| w.buying).fold(Vec::new(), |mut v, k| {
            if !v.contains(&k) {
                v.push(k);
            }
            v
        });
        // Standing orders it raised itself, kept up until they run out.
        o.boosts.retain(|b| tick < b.2);
    }

    /// See to the roots of a chain and park the rest; one note for the lot,
    /// and only when the plan changes or something new was done.
    fn work_chain(&mut self, o: &mut Overseer, c: &Chain, known: Option<&Worry>) -> Worry {
        let tick = self.tick;
        let waits: Vec<Lack> = c.steps.iter().copied().filter(|&l| l != c.top).collect();
        let plan = if waits.is_empty() { String::new() } else { format!("first {}", then_list(&waits)) };
        let mut did: Vec<String> = Vec::new();
        let mut todo = c.roots.clone();
        // What can't wait (thirst, a fey mood) is seen to meanwhile too.
        if c.top.urgency() == 0 && !todo.contains(&c.top) {
            todo.push(c.top);
        }
        for r in todo {
            if o.fixes.iter().any(|f| f.0 == r) {
                continue;
            }
            if let Some(what) = self.fix(o, r) {
                o.fixes.push((r, tick));
                if did.contains(&what) {
                    continue;
                }
                did.push(if r == c.top && !waits.is_empty() && !c.roots.contains(&r) { format!("meanwhile {}", what) } else { what });
            }
        }
        let name = names(&c.tops());
        // Said once, and again only when the plan changes (or long after).
        let changed = known.map_or(true, |w| w.plan != plan) && !(plan.is_empty() && did.is_empty());
        let stale = !did.is_empty() && known.map_or(true, |w| !w.heard(o) || tick >= w.noted + RESAY);
        let short = if known.map_or(false, |w| w.noted > 0) && !changed { "still short" } else { "short" };
        let raw = match (plan.is_empty(), did.is_empty()) {
            (true, _) => format!("Saw the fortress {} of {}: {}", short, name, did.join(" ")),
            (false, true) => format!("Saw the fortress {} of {}: {}.", short, name, plan),
            (false, false) => format!("Saw the fortress {} of {}: {}. {}", short, name, plan, did.iter().map(|d| cap(d)).collect::<Vec<_>>().join(" ")),
        };
        let text = o.voiced(&raw);
        // (Not the very words it said lately, as a plan flickers back and forth.)
        let key = if plan.is_empty() { text.clone() } else { format!("of {}: {}.", name, plan) };
        let echo = o.notes.iter().any(|n| n.1.contains(&key) && tick < n.0 + RESAY);
        let (said, noted) = if (changed || stale) && !echo {
            Self::note_ov(o, tick, text.clone());
            (text, tick)
        } else {
            known.map_or((String::new(), 0), |w| (w.said.clone(), w.noted))
        };
        let now: Vec<Lack> = c.roots.iter().copied().filter(|&r| r != c.top).collect();
        let status = if waits.is_empty() {
            format!("Now: {}", name)
        } else {
            let later: Vec<String> = waits.iter().copied().filter(|l| !now.contains(l)).map(|l| l.what()).chain(std::iter::once(name)).collect();
            format!("Now: {}; then {}", and_list(&now), later.join(", then "))
        };
        Worry { lack: c.top, plan, status, until: 0, buying: None, said, noted, also: c.also.clone() }
    }

    /// A trouble that waits on itself: buy a link of the circle from the
    /// next caravan if one brings it, or give it up for a season. Either
    /// way it is parked till then and said once.
    pub(super) fn break_cycle(&mut self, o: &mut Overseer, tops: &[Lack], cycle: &[Lack]) -> Worry {
        let tick = self.tick;
        let name = names(tops);
        let circle: Vec<String> = (0..cycle.len()).map(|i| format!("no {} without {}", bare(&cycle[i].what()), cycle[(i + 1) % cycle.len()].what())).collect();
        let circle = circle.join(", ");
        let next = SEASONS[((tick / SEASON + 1) % 4) as usize];
        let (raw, status, buying) = match cycle.iter().find_map(|l| l.buyable().map(|k| (*l, k))) {
            Some((l, k)) => {
                if !o.wanted.contains(&k) {
                    o.wanted.push(k);
                }
                (
                    format!("Saw the fortress short of {}, and it's a circle: {}. Will buy {} from the next caravan.", name, circle, l.what()),
                    format!("Waiting on a caravan for {} (for {})", l.what(), name),
                    Some(k),
                )
            }
            None => (
                format!("Gave up on {} for now, it's a circle: {}. Will try again in {}.", name, circle, next.to_lowercase()),
                format!("Gave up on {} till {}", name, next.to_lowercase()),
                None,
            ),
        };
        let text = o.voiced(&raw);
        if !o.notes.iter().any(|n| n.1 == text) {
            Self::note_ov(o, tick, text.clone());
        }
        Worry { lack: tops[0], plan: String::new(), status, until: tick + SEASON, buying, said: text, noted: tick, also: tops[1..].to_vec() }
    }

    /// Keep more of something in stock for a while.
    fn boost(o: &mut Overseer, recipe: Recipe, keep: u32, tick: u64) {
        o.boosts.retain(|b| b.0 != recipe);
        o.boosts.push((recipe, keep, tick + DAY * 20));
    }

    /// Make sure someone does `labor` (`n` dwarves, the idlest first).
    fn hands_for(&mut self, labor: Labor, n: usize) -> bool {
        let have = self.dwarves.iter().filter(|d| d.alive && !d.insane && !d.is_child() && d.has(labor)).count();
        if have >= n {
            return false;
        }
        let mut cands: Vec<(usize, u32, u32)> = self
            .dwarves
            .iter()
            // (Not one who'd have to give up the rival tool: a woodcutter isn't made a miner.)
            .filter(|d| d.alive && !d.insane && !d.is_child() && !d.soldier && !d.has(labor) && labor.exclusive_with().map_or(true, |r| !d.has(r)))
            .map(|d| (crate::dwarf::LABORS.iter().filter(|l| d.has(**l)).count(), u32::MAX - d.skills[labor.idx()], d.id))
            .collect();
        cands.sort();
        for (_, _, id) in cands.into_iter().take(n - have) {
            self.set_labor(id, labor, true);
        }
        true
    }

    /// Do something about `lack`; what was done, in the overseer's words.
    fn fix(&mut self, o: &mut Overseer, lack: Lack) -> Option<String> {
        let tick = self.tick;
        let pop = self.alive_count() as u32;
        match lack {
            Lack::Stone => {
                // Quarry out a storeroom: stone now, room for it after.
                let miners = self.hands_for(Labor::Mining, 2);
                if !o.wants.contains(&RoomUse::Stockpile) {
                    o.wants.insert(0, RoomUse::Stockpile);
                }
                Some(if miners { "set miners to quarry out a storeroom.".into() } else { "had a storeroom quarried out.".into() })
            }
            Lack::Wood => {
                self.hands_for(Labor::Woodcutting, 1);
                let e = self.embark;
                let k = o.done.iter().filter(|d| d.starts_with("chop")).count() as i32;
                let r = 22 + k * 8;
                self.designate(e.offset(-r, -r, -2), e.offset(r, r, 2), Desig::Chop);
                o.done.push(format!("chop{}", k));
                Some("sent woodcutters further out.".into())
            }
            Lack::Blocks => {
                Self::boost(o, Recipe::Blocks, 10 + pop / 2, tick);
                self.hands_for(Labor::Masonry, 1);
                Some("had the masons cut blocks.".into())
            }
            Lack::Mechanism => {
                Self::boost(o, Recipe::Mechanism, 6, tick);
                self.hands_for(Labor::Mechanics, 1);
                Some("ordered mechanisms made.".into())
            }
            Lack::Cage => {
                Self::boost(o, Recipe::WoodCage, 4, tick);
                Some("ordered cages built.".into())
            }
            Lack::Bucket => {
                Self::boost(o, Recipe::Bucket, 3, tick);
                Some("ordered buckets.".into())
            }
            Lack::Ore | Lack::Bars => {
                self.hands_for(Labor::Mining, 2);
                self.ov_mine(o, 30, 30);
                Self::boost(o, Recipe::SmeltOre, 8, tick);
                Some("sent miners after ore.".into())
            }
            Lack::Fuel => {
                Self::boost(o, Recipe::Charcoal, 8, tick);
                self.hands_for(Labor::Woodcutting, 1);
                Some("ordered charcoal burned.".into())
            }
            // Water inside the walls, if a well can be sunk (cistern.rs).
            Lack::Drink if self.start_waterworks(o) => {
                Self::boost(o, Recipe::Brew, 30 + pop * 3, tick);
                self.hands_for(Labor::Brewing, 2);
                Some("set miners to bring water inside the walls, and more hands to brewing.".into())
            }
            Lack::Drink | Lack::Brewable => {
                Self::boost(o, Recipe::Brew, 30 + pop * 3, tick);
                self.hands_for(Labor::Brewing, 2);
                self.hands_for(Labor::Farming, 3 + pop as usize / 6);
                Some("put more hands to farming and brewing.".into())
            }
            Lack::Food => {
                Self::boost(o, Recipe::Cook, 15 + pop * 2, tick);
                self.hands_for(Labor::Cooking, 1);
                self.hands_for(Labor::Farming, 3 + pop as usize / 6);
                self.hands_for(Labor::Fishing, 2);
                self.hunt = o.style != Style::Reckless;
                Some("set more to farming, fishing and the kitchen.".into())
            }
            Lack::Seeds => {
                self.hands_for(Labor::Farming, 2);
                Some("set farmers to gather what grows wild.".into())
            }
            Lack::Thread | Lack::Cloth => {
                Self::boost(o, Recipe::SpinThread, 6, tick);
                Self::boost(o, Recipe::Weave, 4, tick);
                Some("ordered thread spun and cloth woven.".into())
            }
            Lack::Leather => {
                Self::boost(o, Recipe::Tan, 4, tick);
                Some("ordered hides tanned.".into())
            }
            Lack::Pick | Lack::Axe => {
                Self::boost(o, if lack == Lack::Pick { Recipe::ForgePick } else { Recipe::ForgeAxe }, 2, tick);
                Some("ordered tools forged.".into())
            }
            Lack::Manager => {
                let m = self.candidates(super::positions::Position::Manager).first().copied()?;
                self.appoint(super::positions::Position::Manager, Some(m));
                Some("appointed a manager.".into())
            }
            Lack::MoodShop(kind) => {
                // (One already on its way will do.)
                if self.buildings.values().any(|b| b.kind == kind && !b.built) {
                    return None;
                }
                let e = self.embark;
                let p = self.ov_spot(kind, e.offset(4, 4, 0), 24)?;
                self.place_building(kind, p);
                Some("had the workshop the moody dwarf wants built at once.".into())
            }
            Lack::Gem => {
                self.hands_for(Labor::Mining, 2);
                self.ov_mine(o, 30, 40);
                Some("sent miners after gems.".into())
            }
            Lack::Bone => {
                self.hunt = true;
                self.hands_for(Labor::Butchery, 1);
                Some("sent hunters out for bone.".into())
            }
            Lack::Kit(p) => {
                let (recipe, keep) = self.kit_order(p)?;
                Self::boost(o, recipe, keep, tick);
                if recipe == Recipe::LeatherArmor || recipe == Recipe::LeatherHelm {
                    Self::boost(o, Recipe::Tan, keep, tick);
                }
                Some(format!("ordered {} for the soldiers ({}).", p.plural(), recipe.name().to_lowercase()))
            }
            Lack::Doctor => {
                if !self.hands_for(Labor::Medicine, 1) {
                    return None;
                }
                let doc = self.dwarves.iter().filter(|d| d.alive && d.has(Labor::Medicine)).map(|d| d.name.clone()).next()?;
                self.msg(&format!("{} is to tend the wounded.", doc), (150, 220, 200));
                Some(format!("made {} the fortress's doctor.", doc))
            }
            Lack::Workshop(k) => {
                if !o.wants.contains(&RoomUse::Workshops) {
                    o.wants.insert(0, RoomUse::Workshops);
                }
                Some(format!("had a workshop hall dug for the {}.", k.name().to_lowercase()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overseer_sees_to_what_the_fortress_lacks() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        for _ in 0..DAY * 2 {
            g.step();
        }
        // Nobody mines any more, and the masons have no stone.
        let ids: Vec<u32> = g.dwarves.iter().map(|d| d.id).collect();
        for id in ids {
            g.set_labor(id, Labor::Mining, false);
        }
        // (Whatever it noticed in the first days is forgotten.)
        let o = g.overseer.as_mut().unwrap();
        o.notes.clear();
        o.fixes.clear();
        let mut days = 0;
        while days < 6 && !g.overseer.as_ref().unwrap().notes.iter().any(|n| n.1.contains("short of stone:")) {
            g.trouble(Lack::Stone);
            for _ in 0..DAY {
                g.step();
            }
            days += 1;
            eprintln!("day {} miners {} notes {:?}", days, g.dwarves.iter().filter(|d| d.has(Labor::Mining)).count(), g.overseer.as_ref().unwrap().notes.iter().rev().take(3).map(|n| n.1.clone()).collect::<Vec<_>>());
        }
        let o = g.overseer.as_ref().unwrap();
        assert!(o.notes.iter().any(|n| n.1.contains("short of stone:")), "the overseer never noticed: {:?}", o.notes.iter().rev().take(5).collect::<Vec<_>>());
        let miners = g.dwarves.iter().filter(|d| d.alive && d.has(Labor::Mining)).count();
        if miners < 2 {
            for d in g.dwarves.iter() {
                eprintln!("{} alive {} soldier {} insane {} child {} labors {:?}", d.name, d.alive, d.soldier, d.insane, d.is_child(), crate::dwarf::LABORS.iter().filter(|l| d.has(**l)).map(|l| l.short()).collect::<Vec<_>>());
            }
        }
        assert!(miners >= 2, "nobody was set to mine");
        // And a shortage it has seen to isn't fussed over again at once.
        assert!(o.fixes.iter().any(|f| f.0 == Lack::Stone));
    }

    #[test]
    fn a_moody_dwarf_gets_the_workshop_they_need() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        for _ in 0..DAY {
            g.step();
        }
        let kind = BuildingKind::Craftsdwarf;
        g.buildings.retain(|_, b| b.kind != kind);
        let mut days = 0;
        while days < 6 && !g.buildings.values().any(|b| b.kind == kind) {
            g.trouble(Lack::MoodShop(kind));
            for _ in 0..DAY {
                g.step();
            }
            days += 1;
        }
        assert!(g.buildings.values().any(|b| b.kind == kind), "no craftsdwarf's workshop after {} days", days);
        assert_eq!(g.buildings.values().filter(|b| b.kind == kind && !b.built).count().min(1), g.buildings.values().filter(|b| b.kind == kind && !b.built).count(), "only one on its way");
    }

    #[test]
    fn a_fort_without_a_doctor_names_one() {
        let mut g = Game::new(5);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Medicine.idx()] = false;
        }
        let mut o = Overseer::new(Style::Steady);
        assert!(g.fix(&mut o, Lack::Doctor).is_some());
        assert!(g.has_doctor(), "nobody was made a doctor");
        assert_eq!(g.dwarves.iter().filter(|d| d.has(Labor::Medicine)).count(), 1);
        assert!(g.fix(&mut o, Lack::Doctor).is_none(), "one doctor is enough");
    }

    /// A day of the overseer's worrying, with `lacks` announced.
    fn worry_day(g: &mut Game, o: &mut Overseer, lacks: &[Lack]) {
        for &l in lacks {
            g.trouble(l);
        }
        g.ov_troubles(o);
        g.tick += DAY;
    }

    fn said(o: &Overseer, what: &str) -> usize {
        o.notes.iter().filter(|n| n.1.contains(what)).count()
    }

    #[test]
    fn a_chain_is_worked_from_its_root() {
        // No stone and no mason's workshop: blocks wait on both.
        let mut g = Game::new(5);
        g.items.retain(|_, it| it.kind != ItemKind::Boulder);
        g.buildings.retain(|_, b| b.kind != BuildingKind::Mason);
        assert!(g.have(Lack::Pick), "the embark brings picks");
        let c = g.chain(Lack::Blocks);
        assert_eq!(c.steps, vec![Lack::Stone, Lack::Workshop(BuildingKind::Mason), Lack::Blocks]);
        assert_eq!(c.roots, vec![Lack::Stone], "only the stone has nothing to wait on");
        let mut o = Overseer::new(Style::Steady);
        for _ in 0..8 {
            worry_day(&mut g, &mut o, &[Lack::Blocks]);
            if !o.notes.is_empty() {
                break;
            }
        }
        // It quarries first; the blocks wait.
        assert!(o.fixes.iter().any(|f| f.0 == Lack::Stone), "never quarried: {:?}", o.notes);
        assert!(!o.fixes.iter().any(|f| f.0 == Lack::Blocks), "ordered blocks with no stone and no mason");
        assert!(!o.boosts.iter().any(|b| b.0 == Recipe::Blocks));
        assert_eq!(o.notes.len(), 1, "one worry, one note: {:?}", o.notes);
        assert!(o.notes[0].1.contains("short of stone blocks: first stone, then a mason's workshop."), "{}", o.notes[0].1);
        assert_eq!(o.plan(), vec!["Now: stone; then a mason's workshop, then stone blocks".to_string()]);
        // Once there's stone, the workshop is next and the plan says so.
        let at = g.embark;
        g.spawn_item(ItemKind::Boulder, crate::material::Mat::Granite, at);
        assert_eq!(g.chain(Lack::Blocks).roots, vec![Lack::Workshop(BuildingKind::Mason)]);
        worry_day(&mut g, &mut o, &[Lack::Blocks]);
        assert!(o.notes.last().unwrap().1.contains("first a mason's workshop. Had a workshop hall dug for the mason's workshop."), "{:?}", o.notes);
    }

    #[test]
    fn a_circle_is_broken_with_one_note() {
        // No picks, no stone, no metal: stone needs a pick to quarry, and a
        // pick needs bars, which need ore, which needs a pick.
        let mut g = Game::new(5);
        g.items.retain(|_, it| !matches!(it.kind, ItemKind::Boulder | ItemKind::Pick | ItemKind::Bar));
        for d in g.dwarves.iter_mut() {
            d.held = None;
        }
        let c = g.chain(Lack::Mechanism);
        let cycle = c.cycle.clone().expect("a circle");
        assert!(cycle.contains(&Lack::Pick), "{:?}", cycle);
        assert!(c.roots.is_empty(), "nothing in a circle can be worked");
        let mut o = Overseer::new(Style::Steady);
        for _ in 0..12 {
            worry_day(&mut g, &mut o, &[Lack::Mechanism]);
        }
        assert_eq!(said(&o, "circle"), 1, "{:?}", o.notes);
        assert_eq!(o.notes.len(), 1, "{:?}", o.notes);
        assert!(o.notes[0].1.contains("Will buy picks from the next caravan"), "{}", o.notes[0].1);
        assert_eq!(o.wanted, vec![ItemKind::Pick]);
        assert!(o.fixes.is_empty() && o.boosts.is_empty(), "it fretted at the circle: {:?} {:?}", o.fixes, o.boosts);
        assert!(o.plan()[0].starts_with("Waiting on a caravan for picks"), "{:?}", o.plan());
        // The caravan's pick comes: back to work, root first.
        let at = g.embark;
        g.spawn_item(ItemKind::Pick, crate::material::Mat::Iron, at);
        worry_day(&mut g, &mut o, &[Lack::Mechanism]);
        assert!(o.wanted.is_empty());
        assert!(o.fixes.iter().any(|f| f.0 == Lack::Stone), "{:?}", o.notes);
        assert!(o.notes.last().unwrap().1.contains("short of mechanisms: first stone"), "{:?}", o.notes);
    }

    #[test]
    fn a_circle_nobody_sells_is_given_up_for_a_season() {
        let mut g = Game::new(5);
        let mut o = Overseer::new(Style::Steady);
        o.traits = vec![super::super::personality::Trait::Cautious];
        let w = g.break_cycle(&mut o, &[Lack::Cloth], &[Lack::Thread, Lack::Cloth]);
        o.worries.push(w);
        assert_eq!(o.notes.len(), 1);
        let n = &o.notes[0].1;
        assert!(n.starts_with("Uneasy, it gave up on cloth for now"), "{}", n);
        assert!(n.contains("no thread without cloth, no cloth without thread"), "{}", n);
        assert!(o.wanted.is_empty());
        // Parked: not a word, not a fix, all season.
        for _ in 0..20 {
            worry_day(&mut g, &mut o, &[Lack::Cloth]);
        }
        assert_eq!(o.notes.len(), 1, "{:?}", o.notes);
        assert!(!o.fixes.iter().any(|f| f.0 == Lack::Cloth));
        assert!(o.plan()[0].starts_with("Gave up on cloth till"), "{:?}", o.plan());
    }

    #[test]
    fn the_same_worry_is_not_noted_every_day() {
        let mut g = Game::new(5);
        g.items.retain(|_, it| it.kind != ItemKind::Boulder);
        g.buildings.retain(|_, b| b.kind != BuildingKind::Mason);
        let mut o = Overseer::new(Style::Steady);
        for _ in 0..9 {
            worry_day(&mut g, &mut o, &[Lack::Blocks, Lack::Stone]);
        }
        // (The stone is a step of the blocks: one worry.)
        assert_eq!(o.worries.len(), 1, "{:?}", o.worries);
        assert_eq!(said(&o, "short of"), 1, "{:?}", o.notes);
        // It kept quarrying all the while, quietly.
        assert!(o.fixes.iter().any(|f| f.0 == Lack::Stone));
    }

    #[test]
    fn thirst_comes_before_comforts() {
        let mut g = Game::new(5);
        let o = Overseer::new(Style::Steady);
        let lux = [Lack::Cloth, Lack::Bucket, Lack::Bone, Lack::Leather];
        let mut open = lux.to_vec();
        open.push(Lack::Drink);
        let a = g.agenda(&o, &open);
        assert_eq!(a[0].top, Lack::Drink, "{:?}", a.iter().map(|c| c.top).collect::<Vec<_>>());
        // With more worries than it can carry, drink is seen to the first day.
        let mut o = Overseer::new(Style::Steady);
        worry_day(&mut g, &mut o, &open);
        assert!(o.worries.len() <= 3, "{:?}", o.worries);
        assert_eq!(o.worries.first().map(|w| w.lack), Some(Lack::Drink), "{:?}", o.worries);
        assert!(o.notes.first().map_or(false, |n| n.1.contains("short of drink")), "{:?}", o.notes);
        // And a warlord's eye for metal still puts bars before cloth.
        let w = Overseer::new(Style::Warlord);
        let a = g.agenda(&w, &[Lack::Cloth, Lack::Thread, Lack::Bars]);
        assert_eq!(a[0].top, Lack::Bars);
    }

    #[test]
    fn prerequisites_come_from_recipes_and_building_costs() {
        let g = Game::new(5);
        use BuildingKind as K;
        assert_eq!(g.prereqs(Lack::Mechanism), vec![Lack::Stone, Lack::Workshop(K::Mechanic)]);
        assert_eq!(g.prereqs(Lack::Bars), vec![Lack::Ore, Lack::Fuel, Lack::Workshop(K::Smelter)]);
        assert_eq!(g.prereqs(Lack::Fuel), vec![Lack::Wood, Lack::Workshop(K::WoodFurnace)]);
        assert_eq!(g.prereqs(Lack::Pick), vec![Lack::Bars, Lack::Fuel, Lack::Workshop(K::Forge)]);
        assert_eq!(g.prereqs(Lack::Drink), vec![Lack::Brewable, Lack::Workshop(K::Still)]);
        assert_eq!(g.prereqs(Lack::Workshop(K::Carpenter)), vec![Lack::Wood]);
        assert_eq!(g.prereqs(Lack::MoodShop(K::Craftsdwarf)), vec![Lack::Stone]);
        assert_eq!(g.prereqs(Lack::Leather), vec![Lack::Workshop(K::Butcher), Lack::Workshop(K::Tannery)]);
        assert_eq!(g.prereqs(Lack::Stone), vec![Lack::Pick]);
        assert!(g.prereqs(Lack::Food).is_empty());
    }

    #[test]
    fn announcements_say_what_is_short() {
        assert_eq!(Lack::from_desc("a mechanism"), Some(Lack::Mechanism));
        assert_eq!(Lack::from_desc("stone blocks or a boulder"), Some(Lack::Blocks));
        assert_eq!(Lack::from_desc("a stone boulder"), Some(Lack::Stone));
        assert_eq!(Lack::from_desc("a log or blocks"), Some(Lack::Blocks));
        assert_eq!(Lack::from_input(Input::Brewable), Some(Lack::Brewable));
    }
}
