//! The schedule: every system that runs every so many ticks, one line each,
//! in the order they run. A new system adds its line here (in the group for
//! its cadence, or a comment row for its milestone item) rather than editing
//! `Game::step`.

use super::*;

impl Game {
    /// Before the dwarves move: water, the overseer, the fortress's daily round.
    #[rustfmt::skip]
    pub(super) fn schedule_early(&mut self) {
        let tick = self.tick;
        // Fluids (timed as "water").
        let t = self.prof.is_some().then(std::time::Instant::now);
        if tick % 3 == 0 { self.fluids(); }
        if tick % 6 == 0 { self.magma(); }
        if tick % 25 == 0 { self.aquifers(); }
        self.prof_add(0, t);
        if tick % 300 == 75 { self.weather_thoughts(); self.lightning(); }
        if tick % 10 == 5 && self.fire_cells.as_ref().map_or(true, |c| !c.is_empty()) { self.fires(); }
        if tick % 50 == 0 { self.count_fits(); }
        if tick % 50 == 10 { self.annals_tick(); }
        if tick % 25 == 20 { self.army_arrives(); }
        if self.orders_due(tick) { self.manage_orders(); }
        if tick % DAY == DAY / 4 { self.festivals_daily(); }
        if tick % 50 == 25 { self.festival_tick(); }
        // The overseer (timed as "overseer").
        let t = self.prof.is_some().then(std::time::Instant::now);
        if tick % 150 == 7 && self.overseer.is_some() { self.oversee(); }
        if tick % 10 == 3 && self.overseer.is_some() { self.oversee_alert(); }
        self.prof_add(1, t);
        if tick % 300 == 290 { self.prune_designations(); }
        if tick % 25 == 13 { self.miasma_tick(); }
        if tick % 100 == 60 { self.expedition_tick(); self.duels_tick(); self.mysteries_tick(); }
        if tick % DAY == DAY / 2 + 7 { self.drama_daily(); self.pets_daily(); self.milestones_daily(); self.monument_daily(); }
        if tick % 10 == 0 { self.grow_farms(); }
        // M1 (survival):
    }

    /// After the dwarves and creatures have moved: wildlife, the day's
    /// chores, and what each season brings.
    #[rustfmt::skip]
    pub(super) fn schedule_late(&mut self) {
        let tick = self.tick;
        if tick % DAY == DAY / 2 { self.wildlife(); }
        if tick % DAY == 0 {
            self.assign_rooms();
            self.families_daily();
            self.clothes_daily();
            self.weather_daily();
            self.rot_daily();
            self.nobles_daily();
            self.titles_daily();
            self.check_mandate();
            // M1 (survival), daily:
            self.shift_daily();
        }
        if tick % SEASON == 0 { self.season_turns(); }
    }

    /// The turn of a season: news, a new year, migrants, and whatever the
    /// world sends.
    #[rustfmt::skip]
    fn season_turns(&mut self) {
        let season = (self.tick / SEASON) % 4;
        let s = SEASONS[season as usize];
        self.msg(&format!("{} has arrived.", s), (200, 200, 255));
        self.autosave_request = true;
        if season == 0 { self.write_chronicle(); self.new_year(); }
        self.seasonal_ice(season);
        if season == 1 || season == 2 { self.migrants(); }
        self.regrow_shrubs();
        self.maybe_siege();
        self.maybe_megabeast();
        self.maybe_cave_horror();
        self.maybe_thief();
        self.maybe_visitor(season);
        self.maybe_undead();
        self.maybe_stranger();
        self.caravan_season(season);
        self.issue_mandate();
        self.maybe_strange_mood();
        self.word_season();
        // M1 (survival), seasonal:
    }

    /// Each living dwarf, every tick, just after their wounds have bled (and
    /// before `update_dwarf` returns for the unconscious): what the body does
    /// before the mind has a say. One line per item.
    pub(super) fn body_phase(&mut self, d: &mut Dwarf) {
        // M1 (survival): pressing a wound, being carried, a berserk fit.
        self.fit_body(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The refactor into the schedule (and the finder table and the hooks)
    /// changes nothing: a fixed fort's year, recorded before it, comes out
    /// the same.
    #[test]
    fn same_story_after_refactor() {
        let t = run_trial(5003, Style::Steady, 1, &Knobs::default());
        let deaths: Vec<(u64, &str, &str)> = t.deaths.iter().map(|d| (d.tick, d.name.as_str(), d.cause.as_str())).collect();
        assert_eq!(
            deaths,
            vec![
                (254420, "Nothar Alehold", "bled to death"),
                (254580, "Dorna Stonebeard", "bled to death"),
                (254700, "Thekla Deepvein", "bled to death"),
                (255580, "Ragni Granitebrew", "bled to death"),
                (281043, "Grenda Anvilbrew", "thirst"),
                (384689, "Odda Stonefist", "thirst"),
            ]
        );
        let sieges: Vec<(u64, &str, u32, &str)> = t.sieges.iter().map(|s| (s.tick, s.foe.as_str(), s.size, s.outcome.as_str())).collect();
        assert_eq!(sieges, vec![]);
        assert_eq!((t.alive, t.peak_wealth, t.story.len()), (13, 5816, 22));
        assert_eq!(t.ambitions, ["Revel", "Fortify", "Provision", "Expand", "Fortify", "Expand", "Fortify", "Delve"]);
    }

    /// The tasks the survival items will hand out are in place but do nothing
    /// yet: a dwarf given one drops it, and they save and load by name.
    #[test]
    fn the_new_tasks_are_inert() {
        let mut g = Game::new(5);
        let id = g.dwarves[1].id;
        let tasks = [
            Task::FirstAid { patient: id, item: None },
            Task::Rescue { patient: id, bed: Some(7) },
            Task::Press,
            Task::Catatonic,
            Task::Berserk,
            Task::Coax { patient: id },
        ];
        for t in tasks {
            let back: Task = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
            assert_eq!(back, t);
            assert_eq!(t.is_need(), matches!(t, Task::Press | Task::Catatonic));
            let mut d = std::mem::take(&mut g.dwarves[0]);
            d.task = t;
            g.do_task(&mut d);
            assert_eq!(d.task, Task::Idle);
            g.dwarves[0] = d;
        }
    }
}
