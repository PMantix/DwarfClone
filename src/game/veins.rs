//! Following the vein (O10, DIG-11/12): the miners follow a vein of ore or
//! gems the way a player designates one, what digging uncovers is noted as
//! a find, and a strike, a rich vein or something strange in the rock is
//! news. The prospector's report on the first day tells the fort what lies
//! beneath it.

use super::ambitions::{Ambition, Sense};
use super::logkind::LogKind;
use super::memory::MemoryKind;
use super::overseer::{Overseer, Style};
use super::*;
use serde::{Deserialize, Serialize};

/// What a vein designation follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VeinFilter {
    /// One ore (`Some`), or any ore (`None`).
    Ore(Option<Mat>),
    Gem,
}

impl VeinFilter {
    /// Whether rock of this material belongs to the vein.
    pub fn takes(self, m: Mat) -> bool {
        match self {
            VeinFilter::Ore(Some(o)) => m == o,
            VeinFilter::Ore(None) => m.is_ore(),
            VeinFilter::Gem => m.is_gem(),
        }
    }

    /// The filter that follows this one material, if it is ore or a gem.
    pub fn of(m: Mat) -> Option<VeinFilter> {
        if m.is_ore() {
            Some(VeinFilter::Ore(Some(m)))
        } else if m.is_gem() {
            Some(VeinFilter::Gem)
        } else {
            None
        }
    }
}

/// The fort's veins (`Game.veins`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Veins {
    /// Designated tiles that spread along their vein when dug, with its filter.
    #[serde(default)]
    pub follow: Vec<(Pos, VeinFilter)>,
    /// Ore and gems revealed by digging and not yet mined or followed.
    #[serde(default)]
    pub found: Vec<Pos>,
    /// The prospector's report: (material, tiles) near the wagon, ores,
    /// gems, flux and clay; empty until the first morning.
    #[serde(default)]
    pub prospect: Vec<(Mat, u32)>,
    /// The report has been made.
    #[serde(default)]
    pub reported: bool,
    /// The overseer's first look for ore already in sight (older forts
    /// kept no `found` list).
    #[serde(default)]
    pub seeded: bool,
    /// When the last find made a card, so they stay a treat.
    #[serde(default)]
    pub last_card: u64,
}

/// At most this many tiles wait to spread at once (one vein or several).
pub const VEIN_CAP: usize = 200;
/// A vein bigger than this (tiles of the same ore touching) is a rich one.
pub const RICH: usize = 30;
/// Ore and gems kept on the found list.
const FOUND_CAP: usize = 400;

/// Strange things in plain rock, each said as "<name> <text>"; `oddity`
/// gives each its own leavings.
const ODDITIES: [&str; 6] = [
    "cracked open a geode, its heart glittering with amethyst!",
    "found the fossil of a coiled shell-beast set in the {rock}.",
    "dug out an ancient bronze pick, its haft long rotted away. Who mined here before?",
    "broke into a pocket of old bones sealed in the {rock}.",
    "turned up a nugget of native gold in the plain {rock}!",
    "broke into a tiny sealed hollow. The air inside smelled of a thousand years.",
];

impl Game {
    /// Mark `p` to be dug and to spread along its vein (ore of one kind, any
    /// ore, or gems) as it is dug: every touching tile on the level that
    /// passes the filter is designated in turn, up to `VEIN_CAP` waiting.
    /// The API the overseer and the orders (O08) use; `v`/`V` for players.
    pub fn follow_vein(&mut self, p: Pos, f: VeinFilter) -> bool {
        if !self.world.in_bounds(p) || self.veins.follow.len() >= VEIN_CAP {
            return false;
        }
        let t = *self.world.t(p);
        if t.kind != TileKind::Wall || !f.takes(t.mat) || !self.vein_safe(p) {
            return false;
        }
        if t.desig != Desig::Dig && self.designate(p, p, Desig::Dig) == 0 {
            return false;
        }
        if !self.veins.follow.iter().any(|x| x.0 == p) {
            self.veins.follow.push((p, f));
        }
        true
    }

    /// Whether a vein may be followed into `p`: never where digging it would
    /// break into unseen open space (a cavern), and for the overseer only
    /// within its reach of the halls, as `ov_mine` looks.
    fn vein_safe(&self, p: Pos) -> bool {
        let hollow = std::iter::once((0, 0)).chain(DIRS8.iter().copied()).any(|(dx, dy)| self.world.get(p.offset(dx, dy, 0)).map_or(false, |t| !t.revealed && !t.kind.solid()));
        if hollow {
            return false;
        }
        match self.overseer.as_ref().and_then(|o| o.base) {
            // (A delver follows a vein as deep as it runs.)
            Some(b) => p.z <= b.z && (p.z >= b.z - 10 || self.overseer.as_ref().map_or(false, |o| o.style == Style::Delver)) && (p.x - b.x).abs() <= 48 && (p.y - b.y).abs() <= 40,
            None => true,
        }
    }

    /// Follow the veins under every seen tile of a box the player marked.
    pub fn follow_veins_in(&mut self, a: Pos, b: Pos, f: VeinFilter) -> usize {
        let lo = Pos::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let hi = Pos::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
        let mut n = 0;
        for p in box_iter(lo, hi) {
            if self.world.in_bounds(p) && self.world.t(p).revealed && self.follow_vein(p, f) {
                n += 1;
            }
        }
        n
    }

    /// After a dig (`complete_dig`): spread a followed vein to the rock just
    /// laid bare, and note ore and gems that came into sight.
    pub(super) fn vein_dug(&mut self, q: Pos) {
        let filter = self.veins.follow.iter().position(|x| x.0 == q).map(|i| self.veins.follow.swap_remove(i).1);
        let gate = self.overseer.as_ref().and_then(|o| o.base);
        for &(dx, dy) in DIRS8.iter() {
            let p = q.offset(dx, dy, 0);
            let Some(t) = self.world.get(p).copied() else { continue };
            if t.kind != TileKind::Wall || !t.revealed || t.desig != Desig::None || t.built_from.is_some() {
                continue;
            }
            if !(t.mat.is_ore() || t.mat.is_gem()) {
                continue;
            }
            // Never the gatehouse's walls, nor the rock round the waterworks.
            if gate.map_or(false, |b| Self::gatehouse(b, p)) || self.by_waterworks(p) {
                continue;
            }
            match filter {
                Some(f) if f.takes(t.mat) => {
                    self.follow_vein(p, f);
                }
                _ => {
                    if !self.veins.found.contains(&p) {
                        if self.veins.found.len() >= FOUND_CAP {
                            self.veins.found.remove(0);
                        }
                        self.veins.found.push(p);
                    }
                }
            }
        }
    }

    /// Tiles of `m` touching `p` (up and down too), counting `p`, up to `cap`:
    /// the prospector's feel for how big a vein is.
    pub(super) fn vein_size(&self, p: Pos, m: Mat, cap: usize) -> usize {
        let mut seen = HashSet::new();
        let mut stack = vec![p];
        seen.insert(p);
        while let Some(c) = stack.pop() {
            for dz in -1..=1 {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let n = c.offset(dx, dy, dz);
                        if seen.len() >= cap || seen.contains(&n) {
                            continue;
                        }
                        if self.world.get(n).map_or(false, |t| t.mat == m && (t.kind == TileKind::Wall || t.dug)) {
                            seen.insert(n);
                            stack.push(n);
                        }
                    }
                }
            }
        }
        seen.len()
    }

    /// A miner has just dug `q` (which was `was`): strikes, rich veins and
    /// strange finds are news.
    pub(super) fn dug_by(&mut self, d: &mut Dwarf, q: Pos, was: world::Tile) {
        if was.kind != TileKind::Wall || was.desig == Desig::Chop || was.built_from.is_some() {
            return;
        }
        let m = was.mat;
        if m.is_ore() {
            self.count("m2.ore_mined", 1);
        }
        if m.is_ore() || m.is_gem() {
            self.vein_struck(d, q, m);
        } else if m.is_building_stone() {
            self.oddity(d, q, m);
        }
    }

    /// The first tile dug of a cluster (no dug neighbour of the same rock)
    /// is a strike: announced, remembered, and for a rich vein or a fort's
    /// first gems, now and then a card.
    fn vein_struck(&mut self, d: &mut Dwarf, q: Pos, m: Mat) {
        let first_here = (-1..=1).all(|dz| {
            DIRS8.iter().chain(std::iter::once(&(0, 0))).all(|&(dx, dy)| {
                let p = q.offset(dx, dy, dz);
                p == q || self.world.get(p).map_or(true, |t| !(t.dug && t.mat == m))
            })
        });
        if !first_here {
            return;
        }
        let size = self.vein_size(q, m, RICH + 1);
        let rich = m.is_ore() && size > RICH;
        let who = d.name.split(' ').next().unwrap_or(&d.name).to_string();
        let gems_before = self.memory.events.iter().any(|e| e.kind == MemoryKind::Feat && e.text.contains("cluster of"));
        let (rest, color) = if m.is_gem() {
            (format!("has found a cluster of {}s!", m.name()), (220, 140, 255))
        } else if rich {
            (format!("has struck a rich vein of {}!", m.name()), (255, 220, 80))
        } else {
            (format!("has struck a vein of {}!", m.name()), (255, 210, 90))
        };
        self.msg_at(&format!("{} {}", who, rest), color, Some(q), LogKind::Job);
        let weight = if rich || m == Mat::NativeGold { 5 } else if m.is_gem() { 4 } else { 2 };
        self.record(MemoryKind::Feat, &[d.id], Some(q), format!("{} {}", d.name, rest), weight);
        if rich || m.is_gem() || m == Mat::NativeGold {
            d.think(&format!("was thrilled to strike {}", if m.is_gem() { "gems" } else { m.name() }), -8, self.tick);
        }
        let card = (rich || (m.is_gem() && !gems_before) || (m == Mat::NativeGold && size > 8)) && self.tick >= self.veins.last_card + SEASON;
        if card {
            self.veins.last_card = self.tick;
            let (title, body) = if m.is_gem() {
                ("Gems!", format!("{}'s pick has broken into a cluster of {}s, glinting in the lamplight. The gem cutters will have work, and the fort a fortune.", d.name, m.name()))
            } else {
                ("A rich vein", format!("{} has struck a rich vein of {}: more than {} tiles of it run through the rock. The smelters will roar for a year.", d.name, m.name(), RICH))
            };
            self.spotlight(title, &body, Some(q), color);
        }
        // The overseer hears of it: a delver goes after the vein at once,
        // any overseer after a rich one or gems.
        let style = self.overseer.as_ref().map(|o| o.style);
        if let (Some(style), Some(f)) = (style, VeinFilter::of(m)) {
            let follow = style == Style::Delver || rich || m.is_gem();
            // (With the overseer in place, so the vein keeps to its reach.)
            if follow && self.follow_vein_from(q, f) > 0 {
                let what = if m.is_gem() { format!("{}s", m.name()) } else { m.name().to_string() };
                let tick = self.tick;
                if let Some(o) = self.overseer.as_mut() {
                    Self::note_ov(o, tick, format!("Set the miners to follow the {} {} struck.", what, who));
                }
            }
        }
    }

    /// Follow the vein out from a tile already dug: its touching rock that
    /// passes the filter.
    pub(super) fn follow_vein_from(&mut self, q: Pos, f: VeinFilter) -> usize {
        let mut n = 0;
        let gate = self.overseer.as_ref().and_then(|o| o.base);
        for &(dx, dy) in DIRS8.iter() {
            let p = q.offset(dx, dy, 0);
            let ok = self.world.get(p).map_or(false, |t| t.revealed && t.kind == TileKind::Wall && t.desig == Desig::None && t.built_from.is_none());
            if ok && !gate.map_or(false, |b| Self::gatehouse(b, p)) && !self.by_waterworks(p) && self.follow_vein(p, f) {
                n += 1;
            }
        }
        n
    }

    /// Something strange in plain rock, once in a few hundred tiles (by the
    /// place, not the dice).
    fn oddity(&mut self, d: &mut Dwarf, q: Pos, rock: Mat) {
        let h = names::hash(&[0x0dd, self.embark.x as u64, self.embark.y as u64, q.x as u64, q.y as u64, q.z as u64]);
        if h % 1000 >= self.knob("finds.per_mille", 2.0) as u64 {
            return;
        }
        let which = ((h / 1000) % ODDITIES.len() as u64) as usize;
        let rest = ODDITIES[which].replace("{rock}", rock.name());
        let who = d.name.split(' ').next().unwrap_or(&d.name).to_string();
        let text = format!("{} {}", who, rest);
        match which {
            0 => {
                self.spawn_item(ItemKind::Boulder, Mat::Amethyst, q);
            }
            1 => {
                self.spawn_item(ItemKind::Craft, rock, q);
            }
            2 => {
                self.spawn_item(ItemKind::Pick, Mat::Bronze, q);
            }
            3 => {
                self.spawn_item(ItemKind::Bone, Mat::Bone, q);
                self.spawn_item(ItemKind::Bone, Mat::Bone, q);
            }
            4 => {
                self.spawn_item(ItemKind::Boulder, Mat::NativeGold, q);
            }
            _ => {}
        }
        self.msg_at(&text, (200, 230, 160), Some(q), LogKind::Job);
        self.record(MemoryKind::Feat, &[d.id], Some(q), format!("{} {}", d.name, rest), 3);
        if which == 3 {
            d.think("was unsettled by old bones in the rock", 4, self.tick);
        } else {
            d.think("marvelled at a strange find in the rock", -6, self.tick);
        }
        if (h / 7000) % 3 == 0 && self.tick >= self.veins.last_card + SEASON {
            self.veins.last_card = self.tick;
            self.spotlight("A strange find", &format!("Deep in the {}, {}", rock.name(), text), Some(q), (200, 230, 160));
        }
    }

    /// The overseer mines what has been found: ore and gems in sight of its
    /// halls that a miner can stand beside. A delver (or one bent on
    /// prospering or delving) follows each vein; the rest dig what they see.
    pub(super) fn ov_mine(&mut self, o: &Overseer, max: usize, reach: i32) {
        let base = o.base.unwrap();
        if !self.veins.seeded {
            self.veins.seeded = true;
            self.seed_found(o, base, reach.max(30));
        }
        let follow = o.style == Style::Delver || matches!(o.ambition, Some(Ambition::Prosper) | Some(Ambition::Delve));
        // A delver's eye goes further for ore, and sends more after it.
        let (max, reach) = if o.style == Style::Delver { (max * 2, reach + 15) } else { (max, reach) };
        let found = std::mem::take(&mut self.veins.found);
        let mut keep = Vec::new();
        let mut n = 0;
        for p in found {
            let Some(t) = self.world.get(p).copied() else { continue };
            if t.kind != TileKind::Wall || t.desig != Desig::None || !(t.mat.is_ore() || t.mat.is_gem()) || t.built_from.is_some() {
                continue;
            }
            let near = p.z <= base.z && p.z >= base.z - 8 && (p.y - base.y).abs() <= reach && p.x >= base.x - reach && p.x <= base.x + reach + 8 * o.slots;
            // Only what a miner can stand beside: next to ground we've dug ourselves.
            let reachable = DIRS8.iter().any(|&(ex, ey)| self.world.get(p.offset(ex, ey, 0)).map_or(false, |q| q.dug && !q.kind.solid() && q.water <= 3));
            if n >= max || !near || !reachable {
                keep.push(p);
                continue;
            }
            if Self::gatehouse(base, p) || self.by_waterworks(p) {
                continue;
            }
            let done = if follow { VeinFilter::of(t.mat).map_or(false, |f| self.follow_vein(p, f)) } else { self.designate(p, p, Desig::Dig) > 0 };
            if done {
                n += 1;
            } else {
                keep.push(p);
            }
        }
        keep.extend(std::mem::take(&mut self.veins.found));
        self.veins.found = keep;
        // Spread marks whose designation was cancelled go.
        let w = &self.world;
        self.veins.follow.retain(|x| w.t(x.0).desig == Desig::Dig);
    }

    /// Once: what ore and gems are already in sight (a fort from before the
    /// found list, or one that broke into a cavern).
    fn seed_found(&mut self, o: &Overseer, base: Pos, reach: i32) {
        for dz in -8..=0 {
            for dy in -reach..=reach {
                for dx in -reach..=(reach + 8 * o.slots) {
                    let p = base.offset(dx, dy, dz);
                    let Some(t) = self.world.get(p) else { continue };
                    if t.revealed && t.kind == TileKind::Wall && t.desig == Desig::None && (t.mat.is_ore() || t.mat.is_gem()) && !self.veins.found.contains(&p) {
                        self.veins.found.push(p);
                    }
                }
            }
        }
    }

    /// The prospector's report, on the first morning: ores, gems, flux and
    /// clay within 30 tiles of the wagon and 12 levels down.
    pub(super) fn prospect_daily(&mut self) {
        if self.veins.reported {
            return;
        }
        self.veins.reported = true;
        let e = self.embark;
        let mut counts: BTreeMap<Mat, u32> = BTreeMap::new();
        for z in (e.z - 12).max(0)..=e.z {
            for y in (e.y - 30).max(0)..=(e.y + 30).min(self.world.h - 1) {
                for x in (e.x - 30).max(0)..=(e.x + 30).min(self.world.w - 1) {
                    let t = self.world.t(Pos::new(x, y, z));
                    if t.kind == TileKind::Wall && (t.mat.is_ore() || t.mat.is_gem() || matches!(t.mat, Mat::Limestone | Mat::Clay)) {
                        *counts.entry(t.mat).or_insert(0) += 1;
                    }
                }
            }
        }
        self.veins.prospect = counts.into_iter().collect();
        let word = |n: u32| match n {
            0 => "no",
            1..=20 => "a trace of",
            21..=120 => "some",
            121..=400 => "plenty of",
            _ => "a wealth of",
        };
        let parts: Vec<String> = self.veins.prospect.iter().filter(|x| x.0.is_ore() || x.0.is_gem()).map(|&(m, n)| format!("{} {}", word(n), m.name())).collect();
        let flux = self.veins.prospect.iter().any(|x| x.0 == Mat::Limestone);
        let clay = self.veins.prospect.iter().any(|x| x.0 == Mat::Clay);
        let text = if parts.is_empty() {
            "The prospector's report: no ore or gems lie within reach of the wagon.".to_string()
        } else {
            format!("The prospector's report: {}{}{}.", parts.join(", "), if flux { "; limestone for flux" } else { "" }, if clay { "; clay" } else { "" })
        };
        self.msg_at(&text, (230, 200, 120), Some(e), LogKind::Job);
        if let Some(mut o) = self.overseer.take() {
            let ore = self.ore_prospected();
            let verdict = if ore >= 300 { "Rich ground. We will dig deep." } else if ore >= 60 { "Ore enough to work." } else { "Thin ground for a miner." };
            Self::note_ov(&mut o, self.tick, format!("Read the prospector's report. {}", verdict));
            self.overseer = Some(o);
        }
    }

    /// Ore tiles the prospector counted.
    fn ore_prospected(&self) -> u32 {
        self.veins.prospect.iter().filter(|x| x.0.is_ore()).map(|x| x.1).sum()
    }

    /// `Sense::Ore` (an `extra_senses` row): how rich the ground looks, from
    /// the prospector's count and the ore lying found and unmined. It pulls
    /// toward Delve and Prosper.
    pub(super) fn ore_sense(&self) -> (Sense, f64, Vec<(Ambition, f64)>) {
        let report = (self.ore_prospected() as f64 / 300.0).min(1.0);
        let found = (self.veins.found.len() as f64 / 15.0).min(1.0);
        (Sense::Ore, report * 0.6 + found * 0.4, vec![(Ambition::Delve, 1.0), (Ambition::Prosper, 0.5)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slab of unseen granite below the wagon; returns a cell in it.
    fn slab(g: &mut Game) -> Pos {
        let e = g.embark;
        let c = Pos::new(e.x, e.y, e.z - 4);
        for dy in -6..=6 {
            for dx in -6..=12 {
                let t = g.world.t_mut(c.offset(dx, dy, 0));
                t.kind = TileKind::Wall;
                t.mat = Mat::Granite;
                t.revealed = false;
                t.desig = Desig::None;
                t.dug = false;
            }
        }
        g.designations.clear();
        c
    }

    fn dig(g: &mut Game, p: Pos) {
        g.world.t_mut(p).desig = Desig::Dig;
        g.complete_dig(p);
    }

    /// Dig whatever is designated until nothing is left, as miners would.
    fn dig_all(g: &mut Game) {
        for _ in 0..60 {
            let next = g.designations.iter().copied().min();
            let Some(p) = next else { break };
            dig(g, p);
        }
    }

    #[test]
    fn a_vein_designation_spreads_along_the_ore() {
        let mut g = Game::new(5);
        let c = slab(&mut g);
        // A crooked line of hematite running east, with malachite beside it.
        let vein: Vec<Pos> = (0..8).map(|i| c.offset(i, if i >= 4 { 1 } else { 0 }, 0)).collect();
        for &p in &vein {
            g.world.t_mut(p).mat = Mat::Hematite;
        }
        g.world.t_mut(c.offset(0, -1, 0)).mat = Mat::Malachite;
        g.world.t_mut(c).revealed = true;
        assert!(g.follow_vein(c, VeinFilter::Ore(Some(Mat::Hematite))));
        dig_all(&mut g);
        for &p in &vein {
            assert!(g.world.t(p).dug, "{:?} of the vein was dug", p);
        }
        assert!(!g.world.t(c.offset(0, -1, 0)).dug, "the malachite beside it was not followed");
        assert!(!g.world.t(c.offset(1, -1, 0)).dug, "nor the granite");
        assert!(g.veins.found.contains(&c.offset(0, -1, 0)), "but it was noted as found");
        assert!(g.veins.follow.is_empty());
    }

    #[test]
    fn gems_only_follows_only_gems() {
        let mut g = Game::new(5);
        let c = slab(&mut g);
        for i in 0..3 {
            g.world.t_mut(c.offset(i, 0, 0)).mat = Mat::Amethyst;
        }
        g.world.t_mut(c.offset(3, 0, 0)).mat = Mat::Hematite;
        g.world.t_mut(c.offset(0, 1, 0)).mat = Mat::Hematite;
        g.world.t_mut(c).revealed = true;
        g.world.t_mut(c.offset(0, 1, 0)).revealed = true;
        assert!(!g.follow_vein(c.offset(0, 1, 0), VeinFilter::Gem), "ore is not a gem");
        assert!(g.follow_vein(c, VeinFilter::Gem));
        dig_all(&mut g);
        assert!((0..3).all(|i| g.world.t(c.offset(i, 0, 0)).dug));
        assert!(!g.world.t(c.offset(3, 0, 0)).dug && !g.world.t(c.offset(0, 1, 0)).dug);
    }

    #[test]
    fn striking_a_vein_is_announced_once() {
        let mut g = Game::new(5);
        let c = slab(&mut g);
        for dx in 0..6 {
            for dy in -2..=2 {
                g.world.t_mut(c.offset(dx, dy, 0)).mat = Mat::Hematite;
            }
        }
        let mut d = g.dwarves[0].clone();
        let struck = |g: &Game| g.log.iter().filter(|e| e.text.contains("struck") && e.text.contains("hematite")).count();
        let before = struck(&g);
        for dx in 0..4 {
            let p = c.offset(dx, 0, 0);
            let was = *g.world.t(p);
            dig(&mut g, p);
            g.dug_by(&mut d, p, was);
        }
        assert_eq!(struck(&g), before + 1, "one announcement for the vein, not one per tile");
        let e = g.log.iter().rev().find(|e| e.text.contains("struck")).unwrap();
        assert!(e.text.starts_with(d.name.split(' ').next().unwrap()), "{}", e.text);
        assert_eq!((e.at, e.kind), (Some(c), LogKind::Job));
        assert!(g.memory.events.iter().any(|m| m.kind == MemoryKind::Feat && m.text.contains("hematite")));
        assert_eq!(g.annals.counters.get("m2.ore_mined").copied(), Some(4));
        // A new cluster of it elsewhere is a new strike.
        let far = c.offset(10, 0, 0);
        g.world.t_mut(far).mat = Mat::Hematite;
        let was = *g.world.t(far);
        dig(&mut g, far);
        g.dug_by(&mut d, far, was);
        assert_eq!(struck(&g), before + 2);
    }

    #[test]
    fn the_prospector_reports_once() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Delver));
        let n = |g: &Game| g.log.iter().filter(|e| e.text.starts_with("The prospector's report")).count();
        g.prospect_daily();
        g.prospect_daily();
        assert_eq!(n(&g), 1);
        let (_, level, links) = g.ore_sense();
        assert!((0.0..=1.0).contains(&level) && links.iter().any(|l| l.0 == Ambition::Delve));
    }

    /// O10's done-when: from the same seed, a delver brings up at least half
    /// as much ore again as a builder over two seasons, and says it followed a vein.
    #[test]
    fn a_delver_finds_more_ore_than_a_builder() {
        let run = |style: Style| {
            let mut g = Game::new(5);
            g.set_overseer(Some(style));
            for _ in 0..SEASON * 2 {
                g.step();
            }
            let ore = g.annals.counters.get("m2.ore_mined").copied().unwrap_or(0);
            let followed = g.overseer.as_ref().map_or(false, |o| o.notes.iter().any(|n| n.1.contains("to follow the")));
            (ore, followed)
        };
        let (delver, followed) = run(Style::Delver);
        let (builder, _) = run(Style::Builder);
        eprintln!("ore mined in two seasons: delver {} builder {}", delver, builder);
        assert!(delver * 2 >= builder * 3 && delver > 0, "delver {} builder {}", delver, builder);
        assert!(followed, "the delver noted following a vein");
    }
}
