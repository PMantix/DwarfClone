//! Milestone 2's measures (W0-B): what the batch report's "Overseers"
//! section shows. Each M2 package adds its own lines to `m2_stats`, which
//! runs once at the end of a trial and puts its numbers in the annals'
//! counters under `m2.` (so they ride in `Trial.counters` and `--compare`
//! picks them up). Nothing here may draw on the game's random numbers.

use super::*;

impl Game {
    /// At the end of a trial: the fort's M2 numbers into `m2.*` counters,
    /// one line per package (e.g. `self.count("m2.named_places", n)`).
    pub(super) fn m2_stats(&mut self) {
        // P13 O03-a: how smooth the main halls are
        self.hall_stats();
        // P1 O01:
        // P3 O03-b: named places
        // P8 O07-a: reigns, faith at the end
        self.reign_stats();
        // P9 O08-a: chained orders
        // P18 O08-b: workshops with a master (urgent pulls are counted as they happen: priority.rs).
        let masters = self.buildings.values().filter(|b| b.master.is_some()).count() as u32;
        self.count("m2.masters", masters);
        // P14 O04-a: how far the gatehouse has grown (trap kills are counted as they fall: traps.rs).
        let stage = self.overseer.as_ref().map_or(0, |o| o.gate_stage as u32);
        self.count("m2.gate_stage", stage);
        // P25 O12-a: dreams (`m2.inspired`), plans drawn, adopted, finished and ignored are counted as they happen (inspire.rs).
        // P10 O10-a: ore mined (`m2.ore_mined`) is counted as each tile is dug (veins.rs: dug_by).
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save from before Milestone 2 (none of its new fields written)
    /// loads, with them at their defaults; and the fixture save from before
    /// Milestone 1 (`saves.rs`) still does too.
    #[test]
    fn old_saves_load_without_m2_fields() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        g.msg_at("Something happened here.", (255, 255, 255), Some(g.embark), logkind::LogKind::Job);
        g.record(memory::MemoryKind::Other, &[1], None, "a memory".into(), 3);
        let mut v = serde_json::to_value(&g).unwrap();
        let game = v.as_object_mut().unwrap();
        for k in ["memory", "veins", "ledger", "inspirations", "ov_fell"] {
            assert!(game.remove(k).is_some(), "{} is saved", k);
        }
        let o = game["overseer"].as_object_mut().unwrap();
        for k in ["dwarf", "faith", "faith_log", "reigns", "journal", "gate_stage", "orders_seen", "seen_deaths", "seen_sieges", "commands", "adopted"] {
            assert!(o.remove(k).is_some(), "overseer.{} is saved", k);
        }
        for e in game["log"].as_array_mut().unwrap() {
            let e = e.as_object_mut().unwrap();
            e.remove("at");
            e.remove("kind");
        }
        for b in game["buildings"].as_object_mut().unwrap().values_mut() {
            for k in ["name", "master", "min_skill", "pending_links", "timer"] {
                b.as_object_mut().unwrap().remove(k);
            }
        }
        let old: Game = serde_json::from_value(v).unwrap();
        assert!(old.memory.events.is_empty() && old.veins.found.is_empty() && old.ov_fell.is_none());
        let o = old.overseer.as_ref().unwrap();
        assert_eq!((o.faith, o.dwarf, o.gate_stage, o.commands.len()), (60, None, 0, 0));
        assert!(old.log.iter().all(|e| e.at.is_none() && e.kind == logkind::LogKind::Other));
        // A new game's fields round-trip.
        let back: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(back.memory.events.len(), g.memory.events.len());
        assert_eq!(back.log.last().map(|e| (e.at, e.kind)), Some((Some(g.embark), logkind::LogKind::Job)));
        assert_eq!(back.overseer.as_ref().map(|o| o.faith), Some(60));
    }
}
