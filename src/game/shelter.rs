//! The shelter holds: what keeps the alert burrow safe while an army sits
//! outside. The gate's own tiles (the foot of the stair, the doors, the
//! bridge) are no part of it: a dwarf who stopped there when the bridge went
//! up was shut out with the enemy, and a third of those who died "inside
//! the shelter" died on the gate's doorstep that way.

use super::overseer::Overseer;
use super::*;
use serde::{Deserialize, Serialize};

/// What the overseer keeps for the shelter (`Game.shelter`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Shelter {
    /// The civilian alert as it was at the last check, to see it raised.
    #[serde(default)]
    pub was_alert: bool,
}

impl Game {
    /// The shelter's burrow mask, if there is a shelter.
    pub(super) fn shelter_bits(&self) -> Option<u8> {
        self.burrows.iter().find(|b| Some(b.bit) == self.alert_burrow).map(|b| b.mask())
    }

    /// Whether `p` lies in the shelter.
    pub(super) fn in_shelter(&self, p: Pos) -> bool {
        self.shelter_bits().map_or(false, |m| self.world.get(p).map_or(false, |t| t.burrows & m != 0))
    }

    /// The gate's own tiles, outside the drawbridge: the foot of the stair,
    /// the doors and the bridge itself, and the rock round them.
    fn gate_tiles(base: Pos) -> impl Iterator<Item = Pos> {
        box_iter(base.offset(-2, -1, 0), base.offset(2, 2, 0))
    }

    /// Daily (`overseer_hooks.rs`).
    pub(super) fn ov_shelter(&mut self, o: &mut Overseer) {
        let Some(base) = o.base else { return };
        if self.alert_burrow.is_none() {
            return;
        }
        self.gate_out_of_shelter(base);
    }

    /// Every ten ticks (`overseer_hooks.rs`): when the alert is raised, the
    /// shelter is made ready at once.
    pub(super) fn ov_shelter_alert(&mut self, o: &mut Overseer) {
        if self.civ_alert == self.shelter.was_alert {
            return;
        }
        self.shelter.was_alert = self.civ_alert;
        if let (true, Some(base)) = (self.civ_alert, o.base) {
            self.gate_out_of_shelter(base);
        }
    }

    /// Take the gate's tiles out of the shelter, so those running in go on
    /// past the bridge. How many were taken out.
    fn gate_out_of_shelter(&mut self, base: Pos) -> u32 {
        let Some(i) = self.burrows.iter().position(|b| Some(b.bit) == self.alert_burrow) else { return 0 };
        let mask = self.burrows[i].mask();
        let mut n = 0;
        for p in Self::gate_tiles(base) {
            if self.world.get(p).map_or(false, |t| t.burrows & mask != 0) {
                self.world.t_mut(p).burrows &= !mask;
                n += 1;
            }
        }
        self.burrows[i].tiles = self.burrows[i].tiles.saturating_sub(n);
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::overseer::Style;

    /// A Steady fort with its shelter marked (and the overseer taken out).
    pub(crate) fn sheltered_fort() -> (Game, Overseer) {
        for seed in 5000..5010 {
            let mut g = crate::game::trial_game(seed, Style::Steady, 30);
            let o = g.overseer.take().unwrap();
            if o.did("burrow") && o.did("gate") && g.alive_count() >= 5 {
                return (g, o);
            }
        }
        panic!("no Steady fort from 5000 on has its shelter by day 30");
    }

    /// The gate is no part of the shelter, and someone called in from the
    /// stair goes on past the bridge rather than stopping on the doors.
    #[test]
    fn the_gate_is_outside_the_shelter() {
        let (mut g, mut o) = sheltered_fort();
        let base = o.base.unwrap();
        // (As an old save had it: the doors in the shelter.)
        let i = g.burrows.iter().position(|b| Some(b.bit) == g.alert_burrow).unwrap();
        g.burrow_add(i, &[base.offset(1, 0, 0), base.offset(1, 1, 0), base.offset(2, 0, 0)]);
        g.ov_shelter(&mut o);
        for p in Game::gate_tiles(base) {
            assert!(!g.in_shelter(p), "{:?} (base {:?}) is in the shelter", p, base);
        }
        assert!(g.in_shelter(base.offset(4, 0, 0)), "the corridor behind the bridge left the shelter");
        // A civilian on the stair when the alert goes up.
        let id = g.dwarves.iter().find(|d| d.alive && !d.soldier).unwrap().id;
        g.creatures.retain(|_, c| !c.hostile());
        assert_eq!(g.toggle_civ_alert(), Ok(true));
        let di = g.dwarves.iter().position(|d| d.id == id).unwrap();
        g.dwarves[di].pos = base;
        g.dwarves[di].path.clear();
        g.dwarves[di].task = crate::dwarf::Task::Idle;
        for _ in 0..400 {
            g.step();
        }
        let d = &g.dwarves[di];
        assert!(g.in_shelter(d.pos), "{} never got into the shelter: {:?}", d.name, d.pos);
        assert!(!Game::gate_tiles(base).any(|p| p == d.pos), "{} stopped on the gate at {:?}", d.name, d.pos);
    }

    /// Where the drink and food lie in a grown fort: SEED, STYLE, DAYS.
    #[test]
    #[ignore]
    fn supplies_census() {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or(d.into());
        let seed: u64 = env("SEED", "5000").parse().unwrap();
        let style = Style::parse(&env("STYLE", "steady")).unwrap();
        let days: u64 = env("DAYS", "500").parse().unwrap();
        let g = crate::game::trial_game(seed, style, days);
        for (what, f) in [("drink", (|it: &Item| it.kind == ItemKind::Drink) as fn(&Item) -> bool), ("food", |it: &Item| it.is_food())] {
            let mut by: BTreeMap<String, usize> = BTreeMap::new();
            for it in g.items.values().filter(|it| f(it)) {
                let p = it.pos;
                let zone = g.zone_kind_at(p).map_or("no zone".to_string(), |k| format!("{:?}", k));
                let key = format!("{} {} {}", if g.in_shelter(p) { "SHELTER" } else { "outside-shelter" }, if g.world.outside(p) { "sky" } else { "under" }, zone);
                *by.entry(key).or_default() += 1;
            }
            eprintln!("{} (day {}): {:?}", what, g.tick / DAY, by);
        }
    }

    /// How besiegers get into the shelter: run batch forts (SEEDS, comma
    /// separated, styles as the batch gives them) for YEARS and print where
    /// each besieger first stepped into the alert burrow, and every death
    /// inside it.
    #[test]
    #[ignore]
    fn shelter_breach_trace() {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or(d.into());
        let seeds: Vec<u64> = env("SEEDS", "5000").split(',').filter_map(|s| s.trim().parse().ok()).collect();
        let years: u64 = env("YEARS", "2").parse().unwrap();
        for seed in seeds {
            let style = Style::ALL[((seed - 5000) % 5) as usize];
            let mut g = Game::new(seed);
            g.set_overseer(Some(style));
            let mut last: HashMap<u32, Pos> = HashMap::new();
            let mut deaths = 0;
            let mut dead: HashSet<u32> = HashSet::new();
            eprintln!("== seed {} {:?}", seed, style);
            while g.tick < years * YEAR && g.alive_count() > 0 {
                g.step();
                let Some(o) = g.overseer.as_ref() else { break };
                let (Some(top), Some(base)) = (o.top, o.base) else { continue };
                let siege = g.annals.sieges.iter().rposition(|s| s.ended.is_none());
                let ids: Vec<(u32, Pos, String, bool)> = g.creatures.values().filter(|c| c.alive && c.hostile()).map(|c| (c.id, c.pos, crate::game::annals::kind_of(c), c.siege)).collect();
                for (id, p, kind, sieger) in ids {
                    let was = last.insert(id, p);
                    if let Some(q) = was {
                        if g.in_shelter(p) && !g.in_shelter(q) {
                            let raised = g.buildings.values().any(|b| b.kind == BuildingKind::Bridge && b.raised);
                            // The way it came: the nearest open sky from where it stood.
                            let sky = crate::path::search(&g.world, q, 4000, crate::world::Mover::Goblin, |r| g.world.outside(r)).map(|(r, path)| (r, path.len()));
                            eprintln!(
                                "  day {:>4} {} {}{} civ {} in at {:?} from {:?} (rel base {},{},{}) {:?} bridge-up {} sky {:?}",
                                g.tick / DAY, kind, if sieger { "besieger" } else { "beast" }, siege.map_or(String::new(), |s| format!(" siege {}", s)), g.civ_alert as u8,
                                p, q, p.x - base.x, p.y - base.y, p.z - base.z, g.world.t(q).kind, raised, sky
                            );
                        }
                    }
                }
                let newly: Vec<&Dwarf> = g.dwarves.iter().filter(|d| !d.alive && !dead.contains(&d.id)).collect();
                for d in newly {
                    dead.insert(d.id);
                    let killer = d.attacker.and_then(|(cid, _)| g.creatures.get(&cid)).map(|c| (crate::game::annals::kind_of(c), c.pos, g.in_shelter(c.pos)));
                    eprintln!(
                        "  day {:>4} {} died at {:?} (rel base {},{},{}) sheltered-tile {} civ {} soldier {} killer {:?}",
                        g.tick / DAY, d.name, d.pos, d.pos.x - base.x, d.pos.y - base.y, d.pos.z - base.z, g.in_shelter(d.pos), g.civ_alert as u8, d.soldier, killer
                    );
                }
                let _ = top;
                while deaths < g.annals.deaths.len() {
                    let d = &g.annals.deaths[deaths];
                    if d.sheltered {
                        eprintln!("      (annals: {} {} by {:?} siege {:?})", d.name, d.cause, d.killer, d.siege);
                    }
                    deaths += 1;
                }
            }
            eprintln!("  end day {} alive {} sieges {}", g.tick / DAY, g.alive_count(), g.annals.sieges.len());
        }
    }
}
