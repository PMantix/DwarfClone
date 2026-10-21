//! Disasters: halls dug too wide without a pillar bring the ceiling down.
//! Whoever is under it may be buried alive (O10, DIG-07): they lie in the
//! rubble until someone digs them out, or the air runs out.

use super::logkind::LogKind;
use super::memory::MemoryKind;
use super::personality::Trait;
use super::*;
use serde::{Deserialize, Serialize};

/// How long someone lasts under the rubble before the air runs out.
pub const BURIED_FOR: u64 = DAY;
/// The chance that rock falling on someone buries them rather than only
/// wounding them.
const BURY_CHANCE: f64 = 0.6;

/// One dwarf under the rubble.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Buried {
    pub at: Pos,
    pub who: u32,
    pub since: u64,
}

/// Who lies buried where (`Game.rubble`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Rubble {
    /// The buried, the living and the dead, until they are dug out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buried: Vec<Buried>,
    /// Where the rock came down, and when: the miner who brought it down
    /// is caught after the fall (they are out of the list while they act).
    #[serde(skip)]
    pub fell: (u64, Vec<Pos>),
}

/// A freshly dug tile this far from every wall, pillar or undug rock brings
/// the ceiling down; one less and the ceiling creaks in warning.
pub const COLLAPSE_SPAN: i32 = 5;

impl Game {
    /// Every tile within `r` of `q` on its level has been dug out: nothing
    /// holds the ceiling up. Natural caverns don't count; they've stood for
    /// ages.
    fn unsupported(&self, q: Pos, r: i32) -> bool {
        for dy in -r..=r {
            for dx in -r..=r {
                match self.world.get(q.offset(dx, dy, 0)) {
                    Some(t) if t.dug && !t.kind.solid() => {}
                    _ => return false,
                }
            }
        }
        true
    }

    /// After digging out `q`: warn of, or bring down, an unsupported ceiling.
    pub(super) fn check_ceiling(&mut self, q: Pos) {
        if self.world.outside(q) || self.is_rubble(q) || !self.unsupported(q, COLLAPSE_SPAN - 1) {
            return;
        }
        if self.unsupported(q, COLLAPSE_SPAN) {
            self.cave_in(q);
        } else {
            self.count("m2.creaks", 1);
            // Now and then a creaking ceiling doesn't wait for the warning to be heeded.
            if self.rng.chance(self.knob("cavein.creak_fall", 0.04)) {
                self.cave_in(q);
                return;
            }
            self.warn(
                "creak",
                "The ceiling creaks over a wide hall. Leave a pillar of rock or build a wall before you dig further.",
                (240, 180, 90),
            );
        }
    }

    /// The rock overhead comes down around `q`: rubble, and anyone beneath
    /// is crushed.
    pub(super) fn cave_in(&mut self, q: Pos) {
        let tick = self.tick;
        let mut fell = 0;
        self.rubble.fell = (tick, Vec::new());
        for dy in -2..=2 {
            for dx in -2..=2 {
                let p = q.offset(dx, dy, 0);
                let up = p.up();
                let Some(t) = self.world.get(up).copied() else { continue };
                if t.kind != TileKind::Wall || t.built_from.is_some() || t.mat == Mat::Bedrock {
                    continue;
                }
                fell += 1;
                let tt = self.world.t_mut(up);
                tt.kind = TileKind::Open;
                tt.revealed = true;
                if self.rng.chance(0.6) && t.mat.is_building_stone() {
                    self.spawn_item(ItemKind::Boulder, t.mat, p);
                }
                self.rubble.fell.1.push(p);
                for i in 0..self.dwarves.len() {
                    if self.dwarves[i].alive && !self.dwarves[i].buried && self.dwarves[i].pos == p {
                        let mut d = std::mem::take(&mut self.dwarves[i]);
                        self.caught(&mut d, t.mat);
                        self.dwarves[i] = d;
                    }
                }
                let hit: Vec<u32> = self.creatures.values().filter(|c| c.alive && c.pos == p).map(|c| c.id).collect();
                for id in hit {
                    let part = self.creatures[&id].body.pick_part(&mut self.rng);
                    self.creatures.get_mut(&id).unwrap().body.wound(part, 7, false);
                    // Beasts aren't dug out: buried is crushed.
                    if self.rng.chance(BURY_CHANCE) {
                        if let Some(c) = self.creatures.remove(&id) {
                            let name = c.species.info().name;
                            self.msg_at(&format!("The {} was crushed under the falling rock.", name), (220, 150, 90), Some(p), LogKind::Other);
                            self.creature_died(c);
                        }
                    }
                }
            }
        }
        if fell == 0 {
            return;
        }
        self.dig_version += 1;
        self.water_cells = None;
        self.year_log.cave_in = true;
        let tick_msg = "Cave-in! The ceiling of a wide hall has come down. Halls need a pillar every ten tiles.";
        self.msg_at(tick_msg, (255, 120, 60), Some(q), super::logkind::LogKind::Other);
        let fort = self.fortress_name.clone();
        self.legend(format!("The ceiling of a great hall in {} fell in.", fort));
        self.spotlight("Cave-in!", tick_msg, Some(q), (255, 120, 60));
    }
}

impl Game {
    /// Rock comes down on `d` (from `rock`): wounded, and maybe buried.
    fn caught(&mut self, d: &mut Dwarf, rock: Mat) {
        let tick = self.tick;
        let part = d.body.pick_part(&mut self.rng);
        d.body.wound(part, 7, false);
        d.think("was caught in a cave-in", 30, tick);
        let p = d.pos;
        let room = self.world.get(p).map_or(false, |t| t.building == 0 && !t.kind.solid());
        if room && self.rng.chance(BURY_CHANCE) {
            self.bury(d, p, rock);
        }
    }

    /// The miner whose dig brought the ceiling down, if they stood under it.
    pub(super) fn caught_by_fall(&mut self, d: &mut Dwarf) {
        if self.rubble.fell.0 != self.tick || !d.alive || d.buried || !self.rubble.fell.1.contains(&d.pos) {
            return;
        }
        let rock = self.world.get(d.pos.up()).map_or(Mat::Granite, |t| t.mat);
        self.caught(d, rock);
        self.rubble.fell.1.clear();
    }

    /// `d` lies buried under rubble at `p`: the tile fills with rock to be
    /// dug out first of all, and the fort is told.
    pub(super) fn bury(&mut self, d: &mut Dwarf, p: Pos, rock: Mat) {
        let rock = if rock.diggable() && rock != Mat::Air { rock } else { Mat::Granite };
        self.release_task(d);
        d.path.clear();
        d.buried = true;
        let t = self.world.t_mut(p);
        t.kind = TileKind::Wall;
        t.mat = rock;
        t.dug = false;
        t.revealed = true;
        t.desig = Desig::Dig;
        t.water = 0;
        self.designations.insert(p);
        self.dig_version += 1;
        self.rubble.buried.push(Buried { at: p, who: d.id, since: self.tick });
        self.count("m2.buried", 1);
        let first = d.name.split(' ').next().unwrap_or(&d.name).to_string();
        let text = format!(
            "{} is buried alive under the rubble! Dig them out before the air runs out: a day at most. The miners go to it first.",
            d.name
        );
        self.msg_at(&format!("{} is buried alive under the rubble!", d.name), (255, 120, 60), Some(p), LogKind::Other);
        self.spotlight("Trapped!", &text, Some(p), (255, 120, 60));
        self.record(MemoryKind::Wounded, &[d.id], Some(p), "buried alive by a cave-in".to_string(), 6);
        // The overseer sends every spare hand with a pick.
        if self.overseer.is_some() {
            let miners = self.dwarves.iter().filter(|x| x.alive && !x.buried && x.has(Labor::Mining)).count();
            self.hands_for(Labor::Mining, miners + 2);
            let tick = self.tick;
            let o = self.overseer.as_mut().unwrap();
            let text = if o.has(Trait::Sentimental) {
                format!("{} lies under the rubble. Sent every pick we have: I will not lose {} to the rock.", first, first)
            } else {
                format!("Sent extra miners to dig {} out of the rubble.", first)
            };
            Self::note_ov(o, tick, text);
        }
    }

    /// The rubble that buries someone, to be dug first.
    pub(super) fn rubble_digs(&self) -> Vec<Pos> {
        self.rubble.buried.iter().map(|b| b.at).collect()
    }

    /// Whether `p` is rubble over someone (no ceiling to check: it already fell).
    pub fn is_rubble(&self, p: Pos) -> bool {
        self.rubble.buried.iter().any(|b| b.at == p)
    }

    /// A buried dwarf's turn: they wait, and the air runs out.
    pub(super) fn buried_tick(&mut self, d: &mut Dwarf) {
        let Some(b) = self.rubble.buried.iter().find(|b| b.who == d.id).copied() else {
            d.buried = false;
            return;
        };
        if !self.world.solid(b.at) {
            // Dug out by some other way.
            self.unbury(d, None);
            return;
        }
        if self.tick >= b.since + BURIED_FOR {
            self.kill(d, "suffocated under the rubble");
            self.count("m2.buried_died", 1);
        }
    }

    /// `rescuer` has dug out the rubble at `q`: whoever lay under it is free,
    /// or found dead.
    pub(super) fn rubble_dug(&mut self, rescuer: &mut Dwarf, q: Pos) {
        let Some(k) = self.rubble.buried.iter().position(|b| b.at == q) else { return };
        let b = self.rubble.buried.remove(k);
        let Some(i) = self.dwarves.iter().position(|x| x.id == b.who) else { return };
        let mut d = std::mem::take(&mut self.dwarves[i]);
        if d.alive {
            self.unbury(&mut d, Some(rescuer));
        } else {
            d.buried = false;
            let tick = self.tick;
            rescuer.think(&format!("dug out the body of {}", d.name.split(' ').next().unwrap_or("")), 25, tick);
            self.msg_at(&format!("{} has dug out the body of {}.", rescuer.name, d.name), (200, 160, 160), Some(q), LogKind::Death);
        }
        self.dwarves[i] = d;
    }

    /// `d` comes out of the rubble alive.
    fn unbury(&mut self, d: &mut Dwarf, rescuer: Option<&mut Dwarf>) {
        d.buried = false;
        self.rubble.buried.retain(|b| b.who != d.id);
        let tick = self.tick;
        d.think("was dug out of the rubble alive", -20, tick);
        self.count("m2.dug_out", 1);
        let at = d.pos;
        match rescuer {
            Some(r) => {
                r.think(&format!("dug {} out of a cave-in", d.name.split(' ').next().unwrap_or("")), -15, tick);
                let text = format!("{} has dug {} out of the rubble, alive!", r.name, d.name);
                self.msg_at(&text, (150, 230, 150), Some(at), LogKind::Other);
                self.spotlight("Dug out alive!", &format!("{} Hurt and choking on dust, but breathing.", text), Some(at), (150, 230, 150));
                self.record(MemoryKind::Feat, &[r.id, d.id], Some(at), format!("dug {} out of a cave-in alive", d.name), 5);
            }
            None => self.msg_at(&format!("{} has crawled out of the rubble.", d.name), (150, 230, 150), Some(at), LogKind::Other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Carve an open, dug-out square of `r` around `c`, leaving the rock above.
    fn hall(g: &mut Game, c: Pos, r: i32) {
        for dy in -r..=r {
            for dx in -r..=r {
                let p = c.offset(dx, dy, 0);
                let t = g.world.t_mut(p);
                t.kind = TileKind::Floor;
                t.dug = true;
                t.water = 0;
                let up = g.world.t_mut(p.up());
                up.kind = TileKind::Wall;
                up.mat = Mat::Granite;
                up.built_from = None;
            }
        }
    }

    #[test]
    fn a_hall_dug_too_wide_caves_in_and_pillars_hold_it_up() {
        let mut g = Game::new(81);
        let e = g.embark;
        let c = Pos::new(e.x, e.y, g.world.surface_z(e.x, e.y) - 8);
        // Nine wide: it creaks but holds (this time).
        g.knobs.set.insert("cavein.creak_fall".into(), 0.0);
        hall(&mut g, c, COLLAPSE_SPAN - 1);
        g.check_ceiling(c);
        assert!(g.log.iter().any(|l| l.text.contains("creaks")));
        assert_eq!(g.world.t(c.up()).kind, TileKind::Wall);
        // A pillar in the middle of an eleven-wide hall holds it.
        hall(&mut g, c, COLLAPSE_SPAN);
        g.world.t_mut(c.offset(1, 0, 0)).kind = TileKind::Wall;
        g.check_ceiling(c);
        assert_eq!(g.world.t(c.up()).kind, TileKind::Wall, "the pillar should hold it");
        // Without it, down it comes on whoever's underneath.
        g.world.t_mut(c.offset(1, 0, 0)).kind = TileKind::Floor;
        g.dwarves[0].pos = c;
        let before = g.dwarves[0].body.clone();
        g.check_ceiling(c);
        assert_eq!(g.world.t(c.up()).kind, TileKind::Open, "the ceiling should fall");
        assert!(g.log.iter().any(|l| l.text.starts_with("Cave-in!")));
        assert_ne!(g.dwarves[0].body.hp, before.hp, "the dwarf beneath should be hurt");
        assert!(g.items.values().any(|it| it.kind == ItemKind::Boulder && (it.pos.x - c.x).abs() <= 2));
        // Natural caverns stand however wide they are.
        let c2 = c.offset(0, 0, -3);
        hall(&mut g, c2, COLLAPSE_SPAN);
        for dy in -COLLAPSE_SPAN..=COLLAPSE_SPAN {
            for dx in -COLLAPSE_SPAN..=COLLAPSE_SPAN {
                g.world.t_mut(c2.offset(dx, dy, 0)).dug = false;
            }
        }
        g.check_ceiling(c2);
        assert_eq!(g.world.t(c2.up()).kind, TileKind::Wall);
    }

    #[test]
    fn a_buried_dwarf_is_dug_out_alive() {
        let mut g = Game::new(81);
        let e = g.embark;
        let c = Pos::new(e.x, e.y, g.world.surface_z(e.x, e.y) - 8);
        hall(&mut g, c, 3);
        // Seal the hall round with rock, so nobody wanders off.
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                if dx.abs() == 4 || dy.abs() == 4 {
                    let t = g.world.t_mut(c.offset(dx, dy, 0));
                    t.kind = TileKind::Wall;
                    t.mat = Mat::Granite;
                }
            }
        }
        let under = c.offset(1, 0, 0);
        g.dwarves[1].pos = under;
        let mut d = std::mem::take(&mut g.dwarves[1]);
        g.bury(&mut d, under, Mat::Granite);
        g.dwarves[1] = d;
        assert!(g.dwarves[1].buried);
        assert_eq!(g.world.t(under).kind, TileKind::Wall);
        assert!(g.spotlights.iter().any(|s| s.title == "Trapped!"));
        // A miner with a pick, in the hall with them.
        let miner = g.dwarves[0].id;
        g.set_labor(miner, Labor::Mining, true);
        let pick = g.spawn_item(ItemKind::Pick, Mat::Iron, c.offset(-2, 0, 0));
        g.items.get_mut(&pick).unwrap().carried_by = Some(miner);
        g.dwarves[0].held = Some(pick);
        g.dwarves[0].pos = c.offset(-2, 0, 0);
        for _ in 0..BURIED_FOR {
            g.step();
            if !g.dwarves[1].buried {
                break;
            }
        }
        let name = g.dwarves[1].name.clone();
        assert!(!g.dwarves[1].buried, "{} still under the rubble", name);
        assert!(g.dwarves[1].alive, "{} should come out alive", name);
        assert!(g.log.iter().any(|l| l.text.contains("out of the rubble, alive")));
        assert!(g.rubble.buried.is_empty());
    }

    #[test]
    fn nobody_digging_means_the_air_runs_out() {
        let mut g = Game::new(81);
        let e = g.embark;
        let c = Pos::new(e.x, e.y, g.world.surface_z(e.x, e.y) - 8);
        hall(&mut g, c, 1);
        g.dwarves[1].pos = c;
        let mut d = std::mem::take(&mut g.dwarves[1]);
        g.bury(&mut d, c, Mat::Granite);
        g.dwarves[1] = d;
        g.tick += BURIED_FOR;
        g.step();
        assert!(!g.dwarves[1].alive);
        // The body waits to be dug out.
        assert!(g.is_rubble(c));
    }
}
