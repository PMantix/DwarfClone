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
        if tick % gore::SMOKE_TICK == 8 { self.smoke_tick(); }
        if tick % gore::GORE_TICK == 45 { self.gore_tick(); }
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
        if tick % 300 == 290 { self.prune_designations(); self.prune_priorities(); }
        if tick % 25 == 13 { self.miasma_tick(); }
        if tick % 100 == 60 { self.expedition_tick(); self.duels_tick(); self.mysteries_tick(); }
        if tick % DAY == DAY / 2 + 7 { self.drama_daily(); self.pets_daily(); self.milestones_daily(); self.monument_daily(); self.trophies_daily(); }
        if tick % 10 == 0 { self.grow_farms(); }
        // M1 (survival):
        if tick % 150 == 77 { self.doctor_check(); }
        if tick % 50 == 35 { self.watch_siege(); }
        // Myth and magic: the relics' powers at work.
        if tick % 20 == 11 { self.magic_tick(); }
        // Horrors: powers, webs, venom (horrors.rs).
        if tick % horrors::HORROR_TICK == 11 { self.horrors_tick(); }
        // A culture of their own (culture.rs):
        if tick % 100 == 40 { self.culture_tick(); }
        // O09: dwarves crossing an old battlefield (battlefields.rs).
        if tick % 100 == 85 { self.battlefield_tick(); }
        // O12: a dreamer at a table over the plans, and plans gathering dust (inspire.rs).
        if tick % 100 == 90 { self.inspire_tick(); }
        // M3: brawls, bodies found, graves visited (society.rs).
        if tick % 10 == 7 { self.society_tick(); }
        if tick % DAY == DAY / 2 + 11 { self.society_daily(); }
    }

    /// After the dwarves and creatures have moved: wildlife, the day's
    /// chores, and what each season brings.
    #[rustfmt::skip]
    pub(super) fn schedule_late(&mut self) {
        let tick = self.tick;
        if tick % DAY == DAY / 2 { self.wildlife(); }
        if tick % DAY == 0 {
            self.assign_rooms();
            // O09: old battlefields, before the slabs are dedicated (battlefields.rs).
            self.battlefield_daily();
            self.families_daily();
            self.clothes_daily();
            self.weather_daily();
            self.rot_daily();
            self.nobles_daily();
            self.titles_daily();
            self.check_mandate();
            // M1 (survival), daily:
            self.shift_daily();
            // Stories, daily:
            self.stories_daily();
            // Myth and magic (magic.rs), daily:
            self.magic_daily();
            // The thing in the night (haunt.rs).
            self.haunt_daily();
            // Sagas: arcs that end of themselves, the last dwarf (sagas.rs).
            self.sagas_daily();
            // O10: the prospector's report, on the first morning (veins.rs).
            self.prospect_daily();
            // O12: automatons wear and are wound, trinkets find their rooms (clockwork.rs).
            self.clockwork_daily();
            // M4 B01: armies on the march come nearer (worldturn.rs).
            self.world_daily();
        }
        if tick % SEASON == 0 { self.season_turns(); }
        // Interface flair: the highlight reel films its next frame.
        if tick % reel::FRAME_TICKS == 1 { self.reel_tick(); }
        // A04 (squad life): squads at drill in the barracks.
        if tick % 2 == 1 { self.drills_tick(); }
        // Sagas: a death may have been the last (sagas.rs).
        self.sagas_tick();
    }

    /// The turn of a season: news, a new year, migrants, and whatever the
    /// world sends.
    #[rustfmt::skip]
    fn season_turns(&mut self) {
        let season = (self.tick / SEASON) % 4;
        let s = SEASONS[season as usize];
        self.msg(&format!("{} has arrived.", s), (200, 200, 255));
        self.autosave_request = true;
        // O09: a new age and the old tales retold, before the chronicle tells them (tales.rs).
        if season == 0 { self.lore_new_year(); }
        if season == 0 { self.write_chronicle(); self.new_year(); }
        // M4 B17: the world's history rolls on a year, and the fort hears of it (worldturn.rs).
        if season == 0 { self.world_yearly(); }
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
        self.lairs_season(season);
        // Myth and magic, seasonal:
        self.magic_season();
        // O12: a dwarf struck by a vision of a great work (inspire.rs).
        self.inspire_season();
        // A made people's traders in winter (kin.rs).
        self.kin_caravan(season);
    }

    /// Each living dwarf, every tick, just after their wounds have bled (and
    /// before `update_dwarf` returns for the unconscious): what the body does
    /// before the mind has a say. One line per item.
    pub(super) fn body_phase(&mut self, d: &mut Dwarf) {
        // M1 (survival): pressing a wound, being carried, a berserk fit.
        self.check_carried(d);
        self.maybe_press(d);
        self.fit_body(d);
        // Held fast in a web, or stiff from a gaze (horrors.rs).
        self.held_fast(d);
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
        assert_eq!(deaths, vec![(205807, "Dorna Stonebeard", "struck down"), (302569, "Nothar Hammermantle", "struck down"), (302880, "Hilda Frostfist", "bled to death")]);
        let sieges: Vec<(u64, &str, u32, &str)> = t.sieges.iter().map(|s| (s.tick, s.foe.as_str(), s.size, s.outcome.as_str())).collect();
        assert_eq!(sieges, vec![(205695, "goblins", 3, "withdrew")]);
        assert_eq!((t.alive, t.peak_wealth, t.story.len()), (14, 47351, 26));
        assert_eq!(t.ambitions, ["Delve", "Muster", "Revel", "Provision", "Muster", "Revel", "Muster", "Revel", "Provision", "Fortify", "Provision", "Architect"]);
    }

    /// The survival items' tasks save and load by name, and a dwarf given one
    /// with nothing to act on drops it. (FirstAid, Rescue and Press are live
    /// since P7; here the patient isn't bleeding or down and nobody bleeds,
    /// so they drop too. The rest do nothing yet.)
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
