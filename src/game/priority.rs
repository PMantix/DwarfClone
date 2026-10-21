//! Job priorities and postings (O08, ECO-20). A designation or a work
//! order can be marked 1 (drop everything) to 7 (when there's nothing
//! else); unmarked work is 4. Miners take the most urgent dig they can
//! reach first, and every 50 ticks a dwarf busy with something minor (5 or
//! more) drops it when an urgent job (2 or less) of their trade waits with
//! nobody on it. Priorities are rare, so the job search is unchanged while
//! none are set.

use super::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The priority of anything not marked.
pub const NORMAL: u8 = 4;

/// Priorities of designations (`Game.prio`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Priorities {
    /// Designated tiles not at `NORMAL`.
    #[serde(default, serialize_with = "pairs_out", deserialize_with = "pairs_in")]
    pub map: BTreeMap<Pos, u8>,
    /// What new designations get (designate mode's 1-7; 0 = `NORMAL`).
    #[serde(skip)]
    pub new_desigs: u8,
    /// When each urgent tile last pulled a dwarf off other work: once a
    /// day at most, so a dig nobody can reach doesn't empty every bench.
    #[serde(skip)]
    pub pulled: HashMap<Pos, u64>,
}

impl Priorities {
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

fn pairs_out<S: Serializer>(m: &BTreeMap<Pos, u8>, s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(m.iter())
}

fn pairs_in<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<Pos, u8>, D::Error> {
    Ok(Vec::<(Pos, u8)>::deserialize(d)?.into_iter().collect())
}

/// The labor that works a designation.
fn desig_labor(d: Desig) -> Labor {
    match d {
        Desig::Chop => Labor::Woodcutting,
        Desig::Smooth | Desig::Engrave => Labor::Engraving,
        _ => Labor::Mining,
    }
}

impl Game {
    /// A designated tile's priority.
    pub fn prio_of(&self, p: Pos) -> u8 {
        if self.prio.map.is_empty() {
            return NORMAL;
        }
        self.prio.map.get(&p).copied().unwrap_or(NORMAL)
    }

    /// The priorities among `open`, most urgent first (just `NORMAL` while
    /// none are set).
    pub(super) fn prio_tiers(&self, open: &HashSet<Pos>) -> Vec<u8> {
        let mut v = vec![NORMAL];
        if self.prio.map.is_empty() {
            return v;
        }
        for (p, &n) in &self.prio.map {
            if !v.contains(&n) && open.contains(p) {
                v.push(n);
            }
        }
        v.sort();
        v
    }

    /// A work order's priority for the job at the front of a workshop's
    /// queue (`NORMAL` for a job queued by hand).
    pub fn shop_prio(&self, b: &Building) -> u8 {
        let Some(o) = b.orders.first() else { return NORMAL };
        self.work_orders.iter().find(|w| w.recipe == o.recipe && w.mat == o.mat).map_or(NORMAL, |w| w.prio)
    }

    /// Designate as `designate` does, at priority `prio`.
    pub(super) fn designate_prio(&mut self, a: Pos, b: Pos, d: Desig, prio: u8) -> usize {
        let old = std::mem::replace(&mut self.prio.new_desigs, prio);
        let n = self.designate(a, b, d);
        self.prio.new_desigs = old;
        n
    }

    /// After `designate`: mark the box's new designations with the
    /// priority new ones get (or clear old marks there).
    pub(super) fn prio_designated(&mut self, lo: Pos, hi: Pos) {
        let n = match self.prio.new_desigs {
            0 => NORMAL,
            n => n.clamp(1, 7),
        };
        if n == NORMAL && self.prio.map.is_empty() {
            return;
        }
        for p in box_iter(lo, hi) {
            if n != NORMAL && self.designations.contains(&p) {
                self.prio.map.insert(p, n);
            } else {
                self.prio.map.remove(&p);
            }
        }
    }

    /// Forget the marks of work done or cancelled.
    pub(super) fn prune_priorities(&mut self) {
        if !self.prio.map.is_empty() {
            let des = &self.designations;
            self.prio.map.retain(|p, _| des.contains(p));
            let map = &self.prio.map;
            self.prio.pulled.retain(|p, _| map.contains_key(p));
        }
    }

    /// Every 50 ticks: a dwarf on a minor job (5 or more) drops it for an
    /// urgent one (2 or less) of their trade that nobody has taken.
    pub(super) fn urgent_pull(&mut self, d: &mut Dwarf) {
        if self.prio.map.is_empty() || self.tick % 50 != d.id as u64 % 50 {
            return;
        }
        let minor = match d.task {
            Task::Dig { target, .. } => self.prio_of(target) >= 5,
            Task::Craft { bld, .. } => self.buildings.get(&bld).map_or(false, |b| self.shop_prio(b) >= 5),
            _ => false,
        };
        if !minor {
            return;
        }
        let tick = self.tick;
        let urgent = self.prio.map.iter().find(|(p, n)| {
            **n <= 2
                && self.designations.contains(p)
                && !self.claimed.contains(p)
                && d.has(desig_labor(self.world.t(**p).desig))
                && self.prio.pulled.get(p).map_or(true, |&t| tick >= t + DAY)
        });
        let Some((&at, _)) = urgent else { return };
        self.prio.pulled.insert(at, tick);
        self.release_task(d);
        d.task = Task::Idle;
        // Look again at once, whatever the finders found a moment ago.
        self.finder_fail.retain(|k, _| k.0 != d.id);
        self.dig_fail = [None; 4];
        self.count("m2.urgent_pulls", 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_urgent_dig_pulls_a_miner_off_a_minor_one() {
        let mut g = Game::new(42);
        g.creatures.retain(|_, c| c.tame);
        // One miner; the others put their picks down.
        let miner = g.dwarves.iter().find(|d| d.has(Labor::Mining)).map(|d| d.id).expect("a miner");
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Mining.idx()] = d.id == miner;
        }
        // Two boulders of rock on open ground by the wagon: a minor dig
        // near, an urgent one further off, marked once the miner has started.
        let e = g.embark;
        let open = |g: &Game, p: Pos| DIRS8.iter().chain(std::iter::once(&(0, 0))).all(|&(dx, dy)| {
            let q = p.offset(dx, dy, 0);
            g.world.get(q).map_or(false, |t| t.kind == TileKind::Floor && t.building == 0)
        });
        let spots: Vec<Pos> = (-20..=20).flat_map(|dx| (-20..=20).map(move |dy| e.offset(dx, dy, 0))).filter(|&p| open(&g, p)).collect();
        let dist = |q: &Pos| (q.x - e.x).abs().max((q.y - e.y).abs());
        let minor = *spots.iter().filter(|q| dist(q) >= 3).min_by_key(|q| dist(q)).expect("open ground");
        let urgent = *spots.iter().filter(|q| dist(q) >= 10 && dist(&minor) + 6 <= dist(q)).min_by_key(|q| dist(q)).expect("open ground further off");
        for p in [minor, urgent] {
            let t = g.world.t_mut(p);
            t.kind = TileKind::Wall;
            t.mat = Mat::Granite;
            t.revealed = true;
        }
        assert_eq!(g.designate_prio(minor, minor, Desig::Dig, 6), 1);
        assert_eq!(g.prio_of(minor), 6);
        let mut on_minor = false;
        for _ in 0..2000 {
            g.step();
            if matches!(g.dwarves.iter().find(|d| d.id == miner).unwrap().task, Task::Dig { target, .. } if target == minor) {
                on_minor = true;
                break;
            }
        }
        assert!(on_minor, "the miner should take the minor dig while it is all there is");
        assert_eq!(g.designate_prio(urgent, urgent, Desig::Dig, 1), 1);
        let mut pulled = false;
        for _ in 0..200 {
            g.step();
            if matches!(g.dwarves.iter().find(|d| d.id == miner).unwrap().task, Task::Dig { target, .. } if target == urgent) {
                pulled = true;
                break;
            }
        }
        assert!(pulled, "the urgent dig should pull the miner off the minor one");
        assert!(g.designations.contains(&minor), "the minor dig waits");
        // Saved and loaded, the marks stay.
        let back: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back.prio_of(minor), 6);
    }

    #[test]
    fn the_most_urgent_dig_is_taken_first() {
        let mut g = Game::new(42);
        let open: HashSet<Pos> = [g.embark, g.embark.up()].into_iter().collect();
        assert_eq!(g.prio_tiers(&open), vec![NORMAL], "nothing marked: one search, as before");
        g.designations.insert(g.embark);
        g.prio.map.insert(g.embark, 2);
        g.prio.map.insert(g.embark.offset(5, 0, 0), 6);
        assert_eq!(g.prio_tiers(&open), vec![2, NORMAL], "marks off the open digs don't count");
        g.prune_priorities();
        assert_eq!(g.prio.map.len(), 1, "a mark without a designation goes");
    }
}
