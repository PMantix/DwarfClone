//! Records of production (O08, ECO-15): who made what, and when, and what
//! the miners dug out. Written as each craft and each dig is finished; read
//! by the ledger screen (from the orders screen, `l`), the chronicle ("Our
//! smiths forged 14 axes, 9 of them by Kib") and the game-over page.

use super::*;
use serde::{Deserialize, Serialize};

/// One maker's work of one kind in one year.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Made {
    /// The fort's year (0: the year of the embark).
    pub year: u32,
    pub who: u32,
    pub kind: ItemKind,
    pub n: u32,
    /// The best quality among them (0 ordinary .. 5 masterful).
    #[serde(default)]
    pub best: u8,
}

/// One miner's stone of one kind in one year.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mined {
    pub year: u32,
    pub who: u32,
    pub mat: Mat,
    pub n: u32,
}

/// The fort's books (`Game.ledger`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub made: Vec<Made>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mined: Vec<Mined>,
}

/// One row of the ledger screen's makers: a dwarf, how much, of what.
#[derive(Clone, Debug, PartialEq)]
pub struct Maker {
    pub who: u32,
    pub n: u32,
    pub best: u8,
    /// Their kinds, most first.
    pub kinds: Vec<(ItemKind, u32)>,
}

impl Ledger {
    /// What `who` made of `kind` in `year` (0 if nothing).
    pub fn made_by(&self, year: Option<u32>, who: u32, kind: ItemKind) -> u32 {
        self.made.iter().filter(|m| m.who == who && m.kind == kind && year.map_or(true, |y| m.year == y)).map(|m| m.n).sum()
    }

    /// The makers of `year` (or all time), most prolific first.
    pub fn makers(&self, year: Option<u32>) -> Vec<Maker> {
        let mut by: BTreeMap<u32, Maker> = BTreeMap::new();
        for m in self.made.iter().filter(|m| year.map_or(true, |y| m.year == y)) {
            let e = by.entry(m.who).or_insert(Maker { who: m.who, n: 0, best: 0, kinds: Vec::new() });
            e.n += m.n;
            e.best = e.best.max(m.best);
            match e.kinds.iter_mut().find(|k| k.0 == m.kind) {
                Some(k) => k.1 += m.n,
                None => e.kinds.push((m.kind, m.n)),
            }
        }
        let mut v: Vec<Maker> = by.into_values().collect();
        for m in &mut v {
            m.kinds.sort_by(|a, b| b.1.cmp(&a.1));
        }
        v.sort_by(|a, b| b.n.cmp(&a.n).then(a.who.cmp(&b.who)));
        v
    }

    /// Everything made in `year` (or all time) by kind, most first.
    pub fn kinds(&self, year: Option<u32>) -> Vec<(ItemKind, u32)> {
        let mut v: Vec<(ItemKind, u32)> = Vec::new();
        for m in self.made.iter().filter(|m| year.map_or(true, |y| m.year == y)) {
            match v.iter_mut().find(|k| k.0 == m.kind) {
                Some(k) => k.1 += m.n,
                None => v.push((m.kind, m.n)),
            }
        }
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }

    /// Stone dug in `year` (or all time) by material, most first.
    pub fn dug(&self, year: Option<u32>) -> Vec<(Mat, u32)> {
        let mut v: Vec<(Mat, u32)> = Vec::new();
        for m in self.mined.iter().filter(|m| year.map_or(true, |y| m.year == y)) {
            match v.iter_mut().find(|k| k.0 == m.mat) {
                Some(k) => k.1 += m.n,
                None => v.push((m.mat, m.n)),
            }
        }
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }

    /// The miners of `year` (or all time): (who, tiles), most first.
    pub fn miners(&self, year: Option<u32>) -> Vec<(u32, u32)> {
        let mut by: BTreeMap<u32, u32> = BTreeMap::new();
        for m in self.mined.iter().filter(|m| year.map_or(true, |y| m.year == y)) {
            *by.entry(m.who).or_default() += m.n;
        }
        let mut v: Vec<(u32, u32)> = by.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v
    }
}

/// The verb a chronicle uses for a kind of work.
fn verb(kind: ItemKind) -> &'static str {
    if kind.weapon_damage().is_some() || kind.armor_slot().is_some() {
        "forged"
    } else if kind.is_food() {
        "prepared"
    } else {
        "made"
    }
}

impl Game {
    /// The fort's year now, as the ledger counts it.
    pub fn ledger_year(&self) -> u32 {
        (self.tick / YEAR) as u32
    }

    /// A craft is finished: write down what `who` made.
    pub(super) fn ledger_made(&mut self, who: u32, made: &[u32]) {
        let year = self.ledger_year();
        for &id in made {
            let Some(it) = self.items.get(&id) else { continue };
            let (kind, q) = (it.kind, it.quality);
            match self.ledger.made.iter_mut().rev().find(|m| m.year == year && m.who == who && m.kind == kind) {
                Some(m) => {
                    m.n += 1;
                    m.best = m.best.max(q);
                }
                None => self.ledger.made.push(Made { year, who, kind, n: 1, best: q }),
            }
        }
    }

    /// A wall is dug out: write down the stone.
    pub(super) fn ledger_mined(&mut self, who: u32, was: &world::Tile) {
        if was.kind != TileKind::Wall || was.desig == Desig::Chop || was.built_from.is_some() {
            return;
        }
        let (year, mat) = (self.ledger_year(), was.mat);
        match self.ledger.mined.iter_mut().rev().find(|m| m.year == year && m.who == who && m.mat == mat) {
            Some(m) => m.n += 1,
            None => self.ledger.mined.push(Mined { year, who, mat, n: 1 }),
        }
    }

    pub(super) fn first_name_of(&self, id: u32) -> String {
        self.dwarves.iter().find(|d| d.id == id).map_or_else(|| "someone".into(), |d| d.name.split(' ').next().unwrap_or(&d.name).to_string())
    }

    /// The chronicle's lines for the year that ended: the biggest work of
    /// the year and its chief maker, and the year's best miner.
    pub(super) fn lines_ledger(&self) -> Vec<String> {
        let mut v = Vec::new();
        let Some(year) = self.ledger_year().checked_sub(1) else { return v };
        let led = &self.ledger;
        // The kind with the most made, not counting food and drink (what
        // the cooks and brewers make every day is no story).
        let top = led.kinds(Some(year)).into_iter().find(|(k, n)| !k.is_food() && *k != ItemKind::Drink && *n >= 3);
        if let Some((kind, n)) = top {
            let best = led.made.iter().filter(|m| m.year == year && m.kind == kind).map(|m| m.who).max_by_key(|&w| (led.made_by(Some(year), w, kind), u32::MAX - w));
            if let Some(w) = best {
                let by = led.made_by(Some(year), w, kind);
                let tail = if by == n { format!(", every one by {}", self.first_name_of(w)) } else if by * 3 >= n { format!(", {} of them by {}", by, self.first_name_of(w)) } else { String::new() };
                v.push(format!("Our workshops {} {} {}{}.", verb(kind), n, kind.plural(), tail));
            }
        }
        if let Some(&(w, n)) = led.miners(Some(year)).first().filter(|m| m.1 >= 50) {
            v.push(format!("{} dug out {} tiles of rock, more than any other.", self.first_name_of(w), n));
        }
        v
    }

    /// For the game-over page: the fort's greatest maker of all time.
    pub fn ledger_summary(&self) -> Option<String> {
        let m = self.ledger.makers(None).into_iter().next()?;
        let (kind, k) = *m.kinds.first()?;
        let name = self.dwarves.iter().find(|d| d.id == m.who).map_or_else(|| "someone".into(), |d| d.called());
        Some(format!("Its greatest maker was {}, with {} pieces of work ({} {}).", name, m.n, k, kind.plural()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildings::{BuildingKind, Recipe};

    #[test]
    fn who_made_what_is_written_down() {
        let mut g = Game::new(77);
        g.creatures.retain(|_, c| c.tame);
        let site = g.ov_spot(BuildingKind::Mason, g.embark.offset(3, 0, 0), 20).expect("room for a workshop");
        assert!(g.place_building(BuildingKind::Mason, site));
        let id = g.world.t(site).building;
        g.buildings.get_mut(&id).unwrap().built = true;
        g.add_order(id, Recipe::StoneTable);
        g.add_order(id, Recipe::StoneTable);
        for _ in 0..2 {
            g.spawn_item(ItemKind::Boulder, Mat::Granite, site);
        }
        let maker = g.dwarves[1].id;
        for d in g.dwarves.iter_mut() {
            d.labors[crate::dwarf::Labor::Masonry.idx()] = d.id == maker;
        }
        for _ in 0..3 * DAY {
            g.step();
            if g.ledger.made_by(None, maker, ItemKind::Table) >= 2 {
                break;
            }
        }
        assert_eq!(g.ledger.made_by(Some(0), maker, ItemKind::Table), 2, "{:?}", g.ledger);
        let makers = g.ledger.makers(Some(0));
        assert_eq!(makers[0].who, maker);
        assert_eq!(makers[0].kinds[0], (ItemKind::Table, 2));
        // Saved and loaded, the books are the same.
        let back: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back.ledger.made, g.ledger.made);
        // The next year's chronicle tells it.
        g.ledger.made[0].n = 5;
        g.tick += YEAR;
        let lines = g.lines_ledger();
        let first = g.first_name_of(maker);
        assert!(lines.iter().any(|l| l.contains("5 tables") && l.contains(&first)), "{:?}", lines);
    }

    #[test]
    fn the_miners_dig_goes_in_the_books() {
        let mut g = Game::new(42);
        let mut t = *g.world.t(g.embark);
        t.kind = TileKind::Wall;
        t.mat = Mat::Granite;
        t.desig = Desig::Dig;
        t.built_from = None;
        for _ in 0..60 {
            g.ledger_mined(7, &t);
        }
        t.desig = Desig::Chop;
        g.ledger_mined(7, &t);
        assert_eq!(g.ledger.dug(None), vec![(Mat::Granite, 60)]);
        assert_eq!(g.ledger.miners(Some(0)), vec![(7, 60)]);
    }
}
