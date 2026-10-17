# Implementation plan: Milestone 1

Repo: `C:\Users\pmant\source\repos\DwarfClone`. Every file:function reference below was read in the
current tree (October 2026). The numbers come from the newest batch, `batches/o01-50.json`
(48 finished forts out of 50 runs, seeds 5000+, 3 years), recomputed from the JSON with the same
rules as `report.rs` and checked against `--compare batches/troubles.json batches/o01-50.json`.

## Where Milestone 1 stands (batches/o01-50.json, 48 forts)

| What | Now | Milestone goal |
|---|---|---|
| Standing after years 1/2/3 | 42 / 30 / 20 (88% / 63% / 42%) | — |
| Steady standing after 3 years | **3 of 10** | ≥ 40% |
| Warlord standing after 3 years | 9 of 10 | ≥ 40% (met) |
| Builder / Delver / Reckless | 4/9, 4/9, **0/10** | some of each |
| Deaths | 1,063 | — |
| Bled to death | **625 (59%)**, 343 of them in sieges, 429 under the open sky, 103 soldiers | < 25% (S02, S07) |
| Struck down | 355 (33%) | — |
| Fell on an expedition | 25 (2.4%), but **4 forts lost 5–7 dwarves each in a single roll** | < 5% (already met) |
| Thirst / starvation | 21 (20 of them mad) / 30 | no mad dwarf dies of thirst (S09) |
| Killers | goblins 565 (53%), **wolves 190 (18%: 180 bled out, 162 under the sky, only 1 a soldier, up to 14 in one fort, steady in every year: 77/62/51)**, bears 43, cave horrors 33, walking corpses 31 | wildlife deaths halve (S08) |
| Where | 57% during a siege, 63% under the sky, 23% inside the shelter, 15% soldiers | — |
| Sieges (160) | first: 2.6 foes, 2.3 dwarves lost, 1.0 foe slain, bridge up 33%; second: 7.3 / 3.9 / 1.5 / 39%; later: 8.8 / 4.6 / 1.2 / 48%; 18 forts fell to sieges | first < 3 lost (met); later sieges still take forts (met) |
| Blows in sieges | soldiers' melee 2,565 (78% drew blood) · bolts 617 (77%) · foes on soldiers 7,394 (**15%**, armour works) · foes on civilians 9,007 (64%) | soldiers kill ≥ 1 goblin each (S07) |
| Bridge up, by style | **Steady 12%**, Builder 37%, Warlord 59%, Delver 68% | — |

What the numbers say:

1. **Bleeding is still the killer** (59%, up from 45% in the v0.2.0 baseline because forts live
   longer). `body.rs: Body::wound` adds `dmg/3 + 1` to a single `bleeding` value that drops by one
   every 10 ticks, so blood lost grows with the square of the wounds. Nobody bandages a wound
   in the field, nobody carries anyone, and a bleeding dwarf *walks* to a bed
   (`game.rs: choose_task → start_bed(rest=true)`), bleeding all the way.
2. **Soldiers wound goblins but don't kill them.** 78% of soldiers' blows draw blood and yet only
   1.2 goblins die per siege. Siege goblins break at 40% blood (`combat.rs: creature_tick`,
   `breaking`), walk off and heal. Half the outcomes are "withdrew", not "routed". A downed foe
   gives no opening. Crossbows are wooden: swung, they hit for 1.8 (`items.rs: weapon_damage`
   Crossbow 3.0 × `material.rs: weapon_mult` Wood 0.6). Bolts always hit for 9 whatever they're
   made of (`military.rs: try_shoot`).
3. **Wolves kill civilians outdoors, all three years, and the overseer does nothing about it.**
   Wolves move every 2 ticks and dwarves every 3 (`creature.rs` Wolf `move_delay: 2`,
   `game.rs: MOVE_DELAY = 3`), so a civilian who runs is caught. After year 1, 95% of the
   hostile wildlife rolls are a pack (`combat.rs: wildlife`). The overseer's Danger sense only
   fires for legendary creatures (`ambitions.rs: brain`), and the kill order (`military.rs:
   order_kill`) is only ever given by the player.
4. **Steady forts fall to sieges because their bridge is rarely up** (12% of their sieges against
   59–68% for Warlord and Delver; Steady sieges cost 5.4 dwarves on average, Warlord ones 2.0).
   This is a new finding. It needs a `TRIAL_SIEGE=1` trace before S04's "learn" step (below,
   S04 step 0).
5. **Expeditions already meet S06's bar** (2.4% of deaths). What's left is the story problem:
   `expeditions.rs: expedition_returns` makes one roll and kills each member with probability
   `danger × 0.05 × 1.5`, so a squad sent to a lair comes home with nobody. S06 becomes
   "come home beaten rather than dead".
6. **Madness kills by thirst.** 20 of 21 thirst deaths were mad dwarves. Melancholy already can
   lift (`psyche.rs: psyche_tick`), but "stark raving mad" (`do_mood → go_insane`) is permanent
   and only wanders (`game.rs: update_dwarf`, the `sipping` rule covers melancholy only).

---

## Milestone 1: forts that live to tell the tale (v0.3)

**Order and why**

The WISHLIST order is S02, S03, S04, S06, S05, S07, S08, S09, S10. The build order below differs
for three reasons:

- **S10's knobs come first, as a foundation.** S02 (the bleed curve), S03 (siege sizes), S07,
  S08 (wildlife rates) and S04 all need to be tuned by batch. If the constants are knobs before
  those items land, the coordinator can rerun a wave's batch with `--knob bleed.gash=…` instead
  of rebuilding. The design is chosen so that knobs are **declared where they're used**, so the
  foundation never becomes a merge hot spot.
- **S02's bleeding reshape (HLT-03) and S07's "Wounds that leave time to help" (MIL-04) are the
  same change.** Build it once, in S02. S07 keeps only the "openings" half of MIL-04 (a downed
  foe is finished cleanly).
- **S07 and S04 matter more than their place suggests.** Steady fails to sieges, and goblins
  "withdrew" rather than die. Kit, metal bolts and openings (S07) plus learning (S04) go after the
  Steady goal directly, so they run in the same waves as S02, not after it.

Critical path (longest dependency chain):
`P0 plumbing → P1 bleed model → P7 first aid & rescue → P8 hospital ready → wave-3 tuning batch`.
Everything else hangs off P0 and can run alongside.

| Wave | Packages (parallel) | Why this wave |
|---|---|---|
| 0 | **P0** plumbing: hot-spot refactors, knobs, counters (S10 step 1) | Everyone builds on it; one agent, serial |
| 1 | **P1** bleed model (S02.4) · **P2** arms and kit (S07 MIL-01, MIL-02) · **P3** predator sense (S08 CRE-01, -03, -15) · **P4** breakdowns (S09) · **P5** captives (S03) · **P6** wells and cisterns (S05) · **P13** expeditions (S06) | They touch disjoint files after P0 |
| 2 | **P7** first aid and rescue (S02.1–2) · **P8** hospital ready (S02.3, HLT-20/21/47) · **P9** archers and openings (S07 MIL-05, MIL-04b) · **P11** the overseer hunts (S07 MIL-03) · **P12** learning from a siege (S04) | Each needs a wave-1 API |
| 3 | **P14** the rest of wild things (S08 CRE-02, -22, -23, -29) · **P15** path throttling plus the tuning pass (S10 OTH-06; S02 "tuned" check) | Needs P3 and every combat change in place to tune against |

Dependencies on other milestones: none upstream. Downstream, O04 "Defences in the plan" (M2)
should reuse P12's `Lesson` store, and A02 "Blood and nerve" (M5) and MIL-16 (yielding) build
on P5's captive record and P9's openings. B20/B21 (pests, snatchers) reuse P3's predator logic.

**Release check**

- `cargo test --release` green, including every test named below. The ignored arena and
  replay tests (`combat.rs: arena`, `overseer.rs: siege_replay`) are rerun and their tables
  pasted into the release commit message.
- Release batch: `--autoplay --runs 50 --years 3 --seed 5000 --out batches/m1.json`, then
  `--compare batches/o01-50.json batches/m1.json`. Use 50 runs, not 25: with only 10 forts per
  style, one fort is 10 points. Targets:
  - Steady ≥ 4/10 and Warlord ≥ 4/10 standing after year 3. Builder, Delver and Reckless each
    keep ≥ 1 standing, and each still loses ≥ 1 fort.
  - "bled to death" < 25% of deaths (from 59%).
  - Wildlife killers (wolf, bear, lion, jaguar, alligator, scorpion, viper, polar bear) < 50% of
    o01-50's count, scaled per fort.
  - Mad thirst deaths = 0. Expedition deaths < 5%, and no expedition other than to a beast's lair
    loses more than half its members.
  - Still ≥ 3 forts fall to sieges ("losing is fun"), and first sieges still cost < 3 dwarves.
- New counters (P0's `counters`) show in the report: rescues > 0, bandages > 0, wells built in
  ≥ 30% of non-Reckless forts, captives taken in ≥ 20% of forts with cage traps, lessons
  learned ≥ 1 per fort that survives a siege.
- Watch: one `--story` fort per style, read end to end. You should find a rescue, a lesson and
  a hunt in it.

---

### P0 — Plumbing: hot-spot refactors, knobs and counters (S10 step 1: OTH-04)  (effort: 1.5 sessions; risk: low)

This is the foundation for everything else. It is described here and again under S10.

- **Design:**
  - *Knobs declared at the use site.* New `src/game/knobs.rs`:
    ```rust
    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct Knobs { pub set: BTreeMap<String, f64> }          // only what the CLI changed
    impl Game {
        /// A tunable number: `default` unless the batch was run with `--knob name=v`.
        pub fn knob(&self, name: &'static str, default: f64) -> f64;
    }
    pub fn parse_knob(arg: &str) -> Result<(String, f64), String>;   // "bleed.gash=6"
    thread_local! { static SEEN: RefCell<BTreeMap<&'static str, f64>> }  // name → default, for --list-knobs
    ```
    `Game` gets `#[serde(default)] pub knobs: Knobs`. Lookup is a `BTreeMap::get` on a map that
    is nearly always empty, which is cheap. Hot loops (the body tick) copy what they need into a
    small params struct once per day (see P1). Because each item writes
    `self.knob("siege.first_max", 3.0)` in its own code, nobody edits a shared list and there
    is nothing to conflict on.
  - *Counters for the batch report.* `annals.rs: Annals` gets
    `#[serde(default)] pub counters: BTreeMap<String, u32>` plus
    `pub(crate) fn count(&mut self, key: &str, n: u32)` on `Game`. `overseer.rs: Trial` gets
    `#[serde(default)] pub counters: BTreeMap<String, u32>` (copied at the end of `run_trial`)
    and `#[serde(default)] pub knobs: BTreeMap<String, f64>`. `report.rs: report` and `compare`
    print every counter key, summed over forts and as a per-fort average, sorted by name. Old
    batch files load with empty maps.
  - *CLI:* in `main.rs: autoplay_batch`, any number of `--knob name=v` (and `--knobs file.json`,
    a flat `{name: v}` map) is passed through to `run_trial(seed, style, years, &Knobs)`.
    `--list-knobs` runs one 30-day trial and prints `SEEN`.
  - *Hot-spot refactors* (no behaviour change, so `cargo test` and a 5-fort batch are identical):
    1. **`Game::step` → `src/game/schedule.rs`.** Move the periodic calls (game.rs lines
       ~1019–1134) into `fn periodic(&mut self)` in a new file, one `if self.tick % N == K { self.x(); }`
       line per system, grouped by cadence, with a comment row reserved per milestone item. New
       systems add exactly one line there.
    2. **`choose_task`'s finder chain → a table.** `game.rs: choose_task` keeps the needs
       (drink, food, rest, sleep, tantrum, mood, sentence, clothes, child, soldier) and then walks
       `const FINDERS: &[(&str, fn(&mut Game, &mut Dwarf) -> bool)]` in a new
       `src/game/finders.rs`: lever, patient, slaughter, burial, construct, craft, … in today's
       order. Items insert a row (P7 inserts "rescue" before "lever" and "first aid" after it).
    3. **Overseer hooks.** `overseer.rs: ov_daily` ends with `self.ov_daily_hooks(o)` and
       `ov_alert` ends with `self.ov_alert_hooks(o)`, both defined in a new
       `src/game/overseer_hooks.rs` as a plain list of calls. New overseer behaviour (hunts,
       lessons, cisterns, captives' fate, readiness) goes in its own file and adds one line there.
    4. **Brain senses.** `ambitions.rs: brain` gets `self.extra_senses(o, &mut senses, &mut add_links)`.
       New senses live in `overseer_hooks.rs: extra_senses`, as `(Sense, strength)` plus a closure
       giving per-ambition weights. The `Sense` enum gets its new variants appended once, here in
       P0, so wave packages don't collide on the enum: `Lesson`, `Prowlers`, `Unarmed`, `Wounded`
       (see the contracts below).
    5. **Task enum.** Add the new variants now, appended at the end of `dwarf.rs: Task` and
       inert until used: `FirstAid { patient: u32, item: Option<u32> }`,
       `Rescue { patient: u32, bed: Option<u32> }`, `Press`, `Catatonic`, `Berserk`, and
       `Coax { patient: u32 }`. Serde takes variant names, so appending is save-safe, and
       `do_task` gets `_ => self.release_task(d)` arms for them. `ui.rs` task labels get a line
       each. This keeps dwarf.rs, do_task and ui.rs out of the wave packages' way.
- **Steps:**
  1. Schedule, finders and hooks refactor · `game.rs`, new `schedule.rs`, `finders.rs`,
     `overseer_hooks.rs` · test: the existing suite. New `schedule.rs: tests::same_story_after_refactor`
     runs `run_trial(5003, Steady, 1)` and compares the deaths and the siege list with a golden
     list recorded before the refactor, to prove the refactor changes nothing.
  2. Knobs and counters · `knobs.rs`, `annals.rs`, `overseer.rs: Trial, run_trial`, `report.rs`,
     `main.rs` · tests: `knobs::tests::a_knob_changes_the_game` (sets `wildlife.chance=1.0` and
     sees a spawn on day one), `report::tests::counters_are_summed_and_compared`.
  3. Task and Sense variants, inert · `dwarf.rs`, `ambitions.rs`, `ui.rs` · test: a save
     written before the change loads (`saves.rs` round-trip test on a fixture string).
- **Overseer & screen:** none of its own. Knobs used are listed in the batch report header
  ("knobs: bleed.gash=6") so a saved batch says how it was made.
- **Measure:** the 5-fort batch is byte-identical before and after step 1.
- **Risks / open questions:** knob names are free text, so a typo silently uses the default.
  `--list-knobs` and a warning for any `--knob` that was never read by day 30 cover it.
- **Parallel work:** **Writes:** `game.rs: step, choose_task` (bodies moved out), new
  `schedule.rs`, `finders.rs`, `overseer_hooks.rs`, `knobs.rs`; `annals.rs: Annals`;
  `overseer.rs: Trial, run_trial, ov_daily (one line), ov_alert (one line)`;
  `ambitions.rs: Sense, brain (one call)`; `dwarf.rs: Task (variants)`; `report.rs`; `main.rs`.
  **Reads:** nothing. **Hot spots:** all of them, which is why it goes alone in wave 0.
  **Agent size:** one agent, 1–2 sessions. Nothing else starts until it's merged.

---

### S02 Battlefield medicine  (effort: 5 sessions; risk: med)

Covers WISHLIST steps 1–4 and HLT-03, HLT-07, HLT-20, HLT-21 and HLT-47. It is built as three
packages: P1 (bleed model, wave 1), P7 (first aid and rescue, wave 2) and P8 (hospital ready,
wave 2).

- **Design:**
  - **Per-part bleeding (P1, HLT-03 and the reshape half of MIL-04).** `body.rs: Body` gets
    `#[serde(default)] pub bleeds: [i32; 7]`: the bleed rate on each part, in tenths of a unit
    of blood per 10 ticks. It also gets `#[serde(default)] pub pressed: [bool; 7]`,
    `#[serde(default)] pub bandaged: [bool; 7]` and `#[serde(default)] pub blood_frac: i32`, an
    accumulator for the tenths. `bleeding: i32` stays and always holds `bleeds.iter().sum()/10`,
    rounded up, so every `bleeding > 0` check in `medicine.rs`, `ui.rs` and `body.rs` keeps
    working. To migrate an old save, the first `tick()` with `bleeds == [0;7] && bleeding > 0`
    puts `bleeding*10` on UpperBody.
    - `wound(part, dmg, edged)` adds to `bleeds[part]` by result: an edged Bruise adds
      `knob bleed.cut`=3; an edged Gash `bleed.gash`=8; a Mangle `bleed.mangle`=12 (blunt
      too: internal); a Sever `bleed.sever`=45, which doesn't clot; a blunt Bruise or Gash adds
      0. Hits to the upper or lower body are ×1.5 (`bleed.vital`).
    - Each `tick()` (every 10 ticks) loses `sum(bleeds)` tenths of blood. Each part clots by
      `max(1, bleeds[p] * clot / 100)`, where `clot` is `bleed.clot_still`=6 when the body is
      still, 2 when moving or fighting (`moving: bool` is passed in), ×3 when bandaged and ×2
      when pressed. A severed part clots only once bandaged. One gash lying still therefore
      loses about 8/0.06/10 ≈ 13 blood. A severed arm with nobody to help loses 4.5 blood per
      10 ticks, about 230 ticks to death: "minutes". Five gashes while fighting take 400–600
      ticks: "hours", with time to help.
    - New API (contract C1 below): `bleed_rate()`, `worst_bleed()`, `ticks_to_death()`,
      `press(part)`, `bandage(part)`, `suture()`.
    - `Body::tick(&mut self)` becomes `tick_with(&mut self, p: &BleedParams, moving: bool)`, with
      `tick()` kept as `tick_with(&BleedParams::default(), false)` for tests and the arena.
      `Game` caches `BleedParams` in a `#[serde(skip)] bleed_params: Cell<Option<BleedParams>>`
      filled from knobs on first use. Callers: `game.rs: update_dwarf`, where moving is
      `!d.path.is_empty() || d.attack_cd > 0`, and `combat.rs: creature_tick`, where moving is
      `!c.path.is_empty() || c.target.is_some()`.
    - `heal()` doesn't heal while `bleed_rate() > 0` (as today).
  - **Pressing and lying still (P7a, HLT-07).** In `update_dwarf`, after the body tick: a
    non-soldier, or a soldier with no threat adjacent, whose `bleed_rate() >= press_at`
    (`knob bleed.press_at`=6) and with no threat within 5 (`threats_near(d.pos, 5)`) drops
    its task for `Task::Press`. In it the dwarf stands still, `pressed[worst]` is set and
    `moving=false`. The dwarf "lies down for help" if `ticks_to_death() < 600`. They leave
    `Press` when the rate drops below 2, when a threat comes within 5 (they flee: `combat_awareness`
    already sets `Flee`, and Flee is moving), or when a helper takes over. `Press` counts as
    `is_need()` (add it to the matches! in `dwarf.rs`).
  - **Field dressing (P7a).** New finder `find_first_aid` in a new `src/game/rescue.rs`,
    registered in `finders.rs` right after "lever". Any adult non-soldier who has Medicine,
    or who is idle-capable with Hauling, looks within `SEARCH_LIMIT/4` for a dwarf with
    `bleed_rate() > 0` who isn't bandaged on the worst part, isn't in `self.treating`, and has
    no threat within 7. They bring one free Thread, Cloth, or a spare clothing item torn into
    rags (`ItemKind::Shirt | Trousers` lying free); without one they press for the patient.
    The work is 25 ticks (`treat_time`). Then `bandage(worst)`, a thought for both
    ("bandaged X's leg" / "was bandaged by Y in the field", −4), a message, and
    `count("bandages",1)`. Medicine XP +10.
  - **Carrying (P7b, WISHLIST step 2).** `Dwarf` gets `#[serde(default)] pub carried_by: Option<u32>`
    and `#[serde(default)] pub carrying_dwarf: Option<u32>`. New finder `find_rescue` in
    `rescue.rs`, registered before "lever". Candidates are patients who are `unconscious()`,
    or `limp() >= 5`, or in `Press` with `ticks_to_death() < 800`, who are not in a bed and not
    already being carried. It searches only when `threats_near(patient, 9)` is empty. During
    `civ_alert` the rescuer may leave the burrow only if the patient is within 12 tiles of a
    burrow tile and no siege creature is within 20. A doctor goes first; otherwise any adult
    with Hauling, the nearest.
    - `Task::Rescue { patient, bed }`: walk beside the patient, then pick them up
      (`patient.carried_by = Some(d.id)`, `d.carrying_dwarf = Some(patient)`, and the patient
      releases their task). The rescuer walks to the bed at `MOVE_DELAY + 2` per step; the bed
      is a free hospital bed from `medicine.rs: hospital_beds`, else any free bed, else the
      nearest shelter-burrow floor. After each of the rescuer's steps,
      `rescue.rs: carry_dwarf(&mut self, carrier: &Dwarf)` moves the patient (looked up in
      `self.dwarves`; the carrier was taken out with `mem::take`, so this is safe).
    - On arrival: the patient gets `Task::Rest { bed, stand }` with `asleep = true`, the bed's
      occupant is set, `carried_by = None`, and `count("rescues",1)`. The spotlight card
      "Carried from the field" is shown if the patient was unconscious.
    - If the carrier is attacked or flees, the patient is set down: `drop_dwarf`. In
      `update_dwarf`, a dwarf with `carried_by` skips everything but the body tick
      (an early return beside the unconscious branch).
    - Hospital patients don't get up: `Task::Rest` already lasts until `healed()`.
      `find_patient` already treats anyone lying down.
  - **Suture in hospital.** `medicine.rs: do_treat Suture` calls `body.suture()`, which zeroes
    `bleeds` and `pressed`. "Clean" is unchanged.
  - **Hospital before the first siege (P8, WISHLIST step 3, HLT-20).**
    - `overseer.rs: plan()` moves `Hospital` to right after `Dormitory` for Steady, Builder,
      Delver and Warlord. For Reckless: no hospital room, but beds in its dormitory count (no
      change).
    - `ov_labors` always wants `Medicine` 1 (2 at pop ≥ 15) with or without a hospital. Today
      it's only `if hospital zone`.
    - `ov_orders` raises `SpinThread` 4 → 6 and `Splint` 2 → 3, and adds `Weave` 3 at pop ≥ 10.
    - HLT-20: when the Hospital room is fitted (`ov_fit RoomUse::Hospital`), it also adds a
      `ZoneKind::Stockpile(StockFilter::Medical)` on the room's last row. `StockFilter::Medical`
      is appended to the enum and to `StockFilter::ALL`, and accepts Thread, Cloth, Splint,
      Crutch, Soap and Bucket. Treat and FirstAid search prefers items in the hospital
      (`find_item` already takes the nearest).
    - A new trouble `Lack::Doctor` (appended to `troubles.rs: Lack`) fires when someone is
      bleeding or unconscious and no dwarf has Medicine. `fix` grants the labour to the best
      candidate (`hands_for(Labor::Medicine, 1)`).
  - **Booze and meals in bed (P8, HLT-21).** `medicine.rs: Treatment` gets `Feed` appended.
    `find_patient` adds Feed when a lying patient has `hunger > HUNGRY/2` or has had no drink
    for `THIRSTY/2`; supplies are a free Drink or Meal item. Today Water only comes from a well.
    `do_treat` sets `thirst = 0` or `hunger = 0`, and both get thoughts ("brought ale while
    abed", −5; "cared for the wounded", −3). This also lets a wounded dwarf in a fort with no
    well rest in bed (`Task::Rest` releases on `thirst > THIRSTY + 1000`) instead of getting
    up and walking.
  - **Injury readout and medical history (P8, HLT-47).** `Dwarf` gets
    `#[serde(default)] pub med: Vec<(u64, String)>`, capped at 20: "gashed in the left leg by
    a goblin", "bandaged by Urist", "carried to the hospital by…", "sutured by…". Entries come
    from `annals.rs: note_wound` (extended to take the part and the wound) and the treatment
    sites. `ui.rs`: the Health report page (`ui.rs` ~3300, report tab 2) colours each part by
    state: lost = red, mangled = orange, badly wounded = yellow, bleeding rate shown as "▲3".
    The dwarf card shows the last 5 `med` lines.
- **Steps** (each one commit):
  1. **P1-a Per-part bleeding with clotting** · `body.rs`, call sites in `game.rs: update_dwarf`
     and `combat.rs: creature_tick` (one line each) · tests in `body.rs`:
     `one_gash_lying_still_clots` (one Gash, `tick_with(still)` ×300 → alive, blood > 80),
     `a_severed_arm_kills_in_minutes` (Sever, no help → dead between 150 and 350 body ticks),
     `five_gashes_fighting_kill_in_hours` (dead between 400 and 900 ticks while moving, alive
     if bandaged at tick 100), `old_saves_bleed_on_the_upper_body`. Update
     `medicine.rs: bleed_out` (it sets `body.bleeding = 13` directly: switch to
     `bleeds[UpperBody] = 130`).
  2. **P1-b Re-tune and record** · run `cargo test arena -- --ignored --nocapture` before and
     after and put both tables in the commit. Target: a dwarf who loses still loses, but "d
     bled%" for losers falls from 85–100% to ≤ 60%, and fights last 100–200 ticks instead of
     70–100. Fix any test that relied on instant bleed-outs (grep for `bleeding =`).
  3. **P7a Pressing and field dressing** · new `rescue.rs` (find_first_aid, do_first_aid,
     do_press), `finders.rs` (one row), `game.rs: update_dwarf` (one call:
     `self.maybe_press(d)`), `dwarf.rs: is_need` (add Press) · test
     `rescue::tests::friends_bandage_the_wounded_where_they_lie`: three dwarves get
     `wound(LeftLeg, 9, true)` on an `open_patch` with Thread on the ground and nobody hostile.
     Within 300 ticks all three are bandaged, none dies, and `counters["bandages"] >= 3`.
  4. **P7b Carried to bed** · `rescue.rs` (find_rescue, do_rescue, carry_dwarf, drop_dwarf),
     `dwarf.rs` (two fields), `game.rs: update_dwarf` (early return when carried) · tests
     `rescue::tests::the_unconscious_are_carried_to_hospital` (a hospital zone with a bed; a
     dwarf at blood 25% unconscious 15 tiles away ends in that bed within 600 ticks) and
     `rescue::tests::nobody_walks_into_a_fight_to_carry` (a goblin adjacent to the patient
     means no Rescue task). Then the WISHLIST "Done when" test,
     `rescue::tests::three_gashed_dwarves_are_carried_in_and_sutured`: three dwarves gashed
     twice each (`wound(UpperBody, 10, true)` ×2), a hospital with 3 beds, thread, one doctor.
     After a day, all three are in hospital beds with `bleed_rate()==0` and alive.
  5. **P8-a Hospital and doctor before the first siege** · `overseer.rs: plan, ov_labors,
     ov_orders, ov_fit`, `items.rs: StockFilter::Medical`, `troubles.rs: Lack::Doctor` and its
     fix · test `overseer::tests::a_hospital_before_the_first_siege`: `run_trial` keeps the Game
     (expose `trial_game(seed, style, until)`) for 3 styles, and at day 168 each has a fitted
     Hospital room, a Medicine dwarf, and ≥ 2 thread or cloth in stock.
  6. **P8-b Booze for the wounded** · `medicine.rs` (Feed) · test
     `medicine::tests::a_patient_in_bed_is_brought_ale` (no well, a dwarf resting with thirst
     past THIRSTY/2, Drink in stock; a doctor brings it and the patient stays in bed).
  7. **P8-c Injury readout and medical history** · `dwarf.rs` (`med`), `annals.rs: note_wound`,
     `medicine.rs`, `rescue.rs` (push lines), `ui.rs` Health page and dwarf card · test
     `ui` render test with `App::with_game_for_tests`: wound a dwarf, render the Health page,
     and the buffer contains "left leg" and "▲". Plus `medicine::tests::history_records_care`.
  8. **Tuned** (WISHLIST step 4, done in wave 3 with P15): batch with knob sweeps
     `bleed.gash ∈ {6,8,10}` and `bleed.clot_still ∈ {4,6,8}`, 25 forts each, pick the pair
     closest to "bleeding < 25%, sieges still fell ≥ 3 forts", and bake it in as the defaults.
- **Overseer & screen:** the overseer gets `Lack::Doctor` as a trouble. It builds the hospital
  earlier, keeps thread in stock, and with P12 a siege that cost many bleeding dwarves teaches
  it "raise the medical stock" (a lesson, below). Sense `Wounded` (appended in P0; it fires
  when ≥ 2 dwarves are resting injured) pushes Provision +0.6 and Venture −1.5. On screen:
  "Urist presses a hand to the wound in their leg." (once per dwarf per wound, via `warn`),
  "Domas bandages Urist's left leg.", the spotlight card "Carried from the field: Domas carried
  Urist, senseless, out of the gate fight and into the hospital.", and a chronicle line in
  `chronicle.rs`: "N dwarves were carried from the field" when `counters["rescues"]` grew this
  year. The Health page has coloured parts and history.
- **Measure:** batch: "bled to death" from 59% to < 25% (WISHLIST target). Counters: rescues
  ≥ 1 per siege that cost blood, bandages ≥ 3 per siege. Arena: the share of losers who bled
  out ≤ 60%.
- **Risks / open questions:**
  - Goblins bleed by the same model, so **soldiers will kill even fewer goblins** unless P9's
    openings land in the same wave-2 batch. Don't judge P1 alone by siege outcomes.
  - Rescuers walking out to the wounded become victims. Hence "no threats within 9" and the
    burrow rule. Watch "where they died: inside the shelter".
  - Decision for the user: may bandages be torn from spare clothing (always available), or
    only thread and cloth (needs the craftsdwarf's shop and fibre)? Recommend: clothing too,
    with a "tore up a shirt for bandages" line.
  - Carrying makes one dwarf follow another; any code that sets `d.pos` for a carried dwarf
    (flee, falling) must check `carried_by` first. Grep `\.pos = ` in `game.rs` and
    `combat.rs`.
- **Parallel work:**
  - **P1:** **Writes** `body.rs` (Body fields, wound, tick_with, new fns), `game.rs:
    update_dwarf` (1 line), `combat.rs: creature_tick` (1 line), `medicine.rs: do_treat`
    (Suture → `suture()`), `medicine.rs: tests::bleed_out`. **Reads** P0 (knob).
    **Hot spots:** two one-line call changes. **Agent size:** one agent, 1–2 sessions.
  - **P7:** **Writes** new `rescue.rs`, `finders.rs` (2 rows), `game.rs: update_dwarf` (2
    lines), `dwarf.rs` (2 fields, `is_need`). **Reads** P1 contract C1
    (`press`, `bandage`, `bleed_rate`, `ticks_to_death`, `worst_bleed`), P0 Task variants.
    **Agent size:** 2–3 sessions; split P7a (press and dress) and P7b (carry) across two agents
    one after the other, or one agent over two sessions.
  - **P8:** **Writes** `overseer.rs: plan, ov_labors, ov_orders, ov_fit (Hospital arm)`,
    `items.rs: StockFilter`, `troubles.rs: Lack, fix (one arm)`, `medicine.rs: Treatment,
    find_patient, do_treat`, `ui.rs` Health page, `dwarf.rs: med`, `annals.rs: note_wound`.
    **Reads** P1 (`bleed_rate`) for the readout. **Hot spots:** overseer.rs (4 small edits,
    serialize with P11 and P12, which don't touch these functions) and ui.rs (one page).
    **Agent size:** one agent, 2 sessions. P8-c can go to a third agent if needed.

---

### S03 Sieges that build up: captives  (effort: 1.5 sessions; risk: low)

Only the last step is open. MIL-16 (yielding) and ECO-18 (selling captives) build on it later.

- **Design:** today `traps.rs: spring_trap` (CageTrap arm) deletes the creature and leaves a
  `Cage` item with `label = Some(species name)`. Nothing else happens, and a siege half caught
  in cages **doesn't rout**: the rout check is only in `combat.rs: creature_died`.
  - New `src/game/captives.rs`:
    ```rust
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Captive { pub cage: u32, pub species: Species, pub name: String, pub civ: Option<u32>,
                         pub siege: Option<usize>, pub caught: u64, pub fate: Fate }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Fate { Held, Questioned, Ransomed, Freed, Executed, Escaped }
    ```
    `Game` gets `#[serde(default)] pub captives: Vec<Captive>`. Siege goblins get a person name
    (`overworld::person_name`) when caught. A general caught is a famous captive.
  - `spring_trap` (CageTrap) calls `self.take_captive(&c, cage)` for creatures with
    `c.siege || c.species.person()`. Animals stay as they are (tameable cages). That call
    records the captive, bumps a new `SiegeNote.foes_caught` (`#[serde(default)]`), and runs
    the rout check. The rout check moves out of `creature_died` into
    `combat.rs: check_rout(&mut self)`, called from both places. Catching the general routs
    the army, as killing them does.
  - **The overseer decides their fate**, once a day, in `captives.rs: ov_captives` (registered in
    `overseer_hooks.rs: daily`), for each `Held` captive older than 2 days and with no siege on:
    - **Questioned** (Cautious, Steady, or the default when the enemy's civ is alive): sets
      `self.siege_warned = true`. It already exists ("a spy's report brings them for
      certain"), so the next siege comes with the warning a season early, and the scouts'
      text adds "as the prisoner {name} let slip". Then Held again (max one question each).
    - **Ransomed** at the next caravan of their civ, or of any civ if it's a general: gold bars
      worth 300 (×3 for a general), a note, the cage freed.
    - **Executed** (Bold, Vain, or the captive is a general and the overseer is Warlord):
      every dwarf thinks "saw the goblin {name} put to death" (−5 Bold and Vain overseers,
      +3 for dwarves with `personality.anxiety > 25`), and the next siege of that civ
      gets +2 size: `Annals.grudges: BTreeMap<u32, u32>` (`#[serde(default)]`), read in
      `combat.rs: siege_strength`, which says "come to avenge {name}".
    - **Freed** (Sentimental, Cheerful): the next siege of that civ is −1 size and says
      "they remember {name} was spared".
  - **Comrades free them:** in `combat.rs: creature_think` (siege branch), before targeting
    dwarves, a goblin whose civ has a `Held` captive in a reachable cage goes to it, gets the
    `bash` treatment (250 ticks), and the captive comes back as a hostile siege creature
    (`Fate::Escaped`), with "freed {name} from the cage!". It's one extra branch in the
    siege think, gated by `!self.captives.is_empty()`.
- **Steps:**
  1. Captives taken, and they count toward a rout · `captives.rs` (record, take_captive),
     `traps.rs: spring_trap` (1 call), `combat.rs: check_rout` (extracted), `annals.rs:
     SiegeNote.foes_caught`, `report.rs` (a "caught" column) · test
     `captives::tests::a_siege_half_caged_routs`: force_siege with 4 goblins, cage-trap two
     (call `spring_trap` directly). The captives are named, `siege_routed` is true, and the log
     has "caught".
  2. The overseer decides their fate · `captives.rs: ov_captives`, `overseer_hooks.rs` (1 line),
     `combat.rs: siege_strength, maybe_siege` (grudge and "let slip" text) · tests: a Bold overseer
     executes and the next `siege_strength()` is +2; a Cautious one questions and
     `siege_warned` is set.
  3. Besiegers free their own · `combat.rs: creature_think` siege branch (1 guarded call into
     `captives.rs: rescue_target`) · test: one goblin, one cage with a held goblin captive on an
     `open_patch`, no dwarves. Within 400 ticks the captive is `Escaped` and a second siege
     creature exists.
- **Overseer & screen:** a captives line on the Military (squads) panel: "In cages: Snaga the
  goblin (held), Ushat (questioned)". Announcements and a spotlight for a caught general
  ("The general Ushat is in a cage!"). Chronicle: "The goblin X was put to death / ransomed /
  set free". Counters: `captives`, `executed`, `ransomed`, `freed`.
- **Measure:** batch: captives taken in ≥ 20% of forts with cage traps. "Withdrew" outcomes fall
  in favour of "routed". Executing forts see bigger next sieges (check with `--story`).
- **Risks / open questions:**
  - Execution is a dark beat. It's kept off-screen (a thought and a chronicle line), which fits
    the house tone. The user should confirm.
  - Ransom needs a caravan of that civ. Goblins don't trade, so goblins are only questioned,
    executed or freed.
- **Parallel work:** **Writes** new `captives.rs`; `traps.rs: spring_trap` (1 line);
  `combat.rs: creature_died` (rout check moved to `check_rout`), `siege_strength`,
  `maybe_siege` (text), `creature_think` siege branch (1 guarded call); `annals.rs: SiegeNote`.
  **Reads** P0 hooks and counters. **Hot spots:** `Game` (+1 field), combat.rs (shared with P3,
  but P3 edits the non-siege branch of `creature_think`: tell both agents to keep to their
  branch), `overseer_hooks.rs` (1 line). **Agent size:** one agent, 1–2 sessions.

---

### S04 An overseer that defends: learning from a siege (SIE-01)  (effort: 2 sessions; risk: med)

- **Design:**
  - **Step 0: why Steady's bridge is up in only 12% of its sieges.** Run
    `TRIAL_SIEGE=1 --autoplay --runs 10 --style steady --seed 5000 --years 3` and read the
    `done [...] | mechanisms | gate parts` lines. Suspects, in order:
    - the gate corridor isn't dug when the plan's first rooms are (`ov_defences` waits for
      `dug_share(base+1..base+6) >= 1.0`);
    - mechanisms are spent on cage traps first (`ov_orders` keeps only 5);
    - the lever isn't linked because one bridge half is unbuilt (`gate_bridges` needs 2).
    Fix whatever it shows in the same package. It has a direct effect on the milestone's
    Steady goal.
  - **What a siege teaches.** New `src/game/lessons.rs`. While a siege is on (`annals_tick`
    has the open SiegeNote), `lessons.rs: watch_siege` runs every 50 ticks (one line in
    `schedule.rs`) and keeps a `SiegeWatch` (`#[serde(skip)]` on Game is fine; if lost on load,
    you just learn less):
    `{ deaths_outside: u32, deaths_inside: u32, foes_inside: bool, bridge_up_at: Option<u64>,
    first_contact: Option<Pos>, kill_spots: Vec<Pos>, soldiers_lost: u32 }`.
    `kill_spots` are where dwarves fell, from `note_death`. `foes_inside` means a siege
    creature stood on a shelter-burrow tile.
  - When the siege closes (`annals_tick` sets `ended`), `lessons.rs: learn(&mut self, o)`
    turns the watch into one or more lessons, stored on the overseer:
    ```rust
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Lesson { TrapTheWay, SecondSquad, WallTheYard, BarSooner, ArmBetter, MoreMedicine }
    ```
    `Overseer` gets `#[serde(default)] pub lessons: Vec<(u64, Lesson)>`. The rules:
    - foes got inside, or the bridge was down at first contact → `TrapTheWay` (weapon traps
      on `kill_spots` within 15 of base, and stone traps, using the existing placement in
      `ambitions.rs: pursue Fortify`, plus `BarSooner`: patience −60 ticks per lesson, floor 30);
    - ≥ 2 deaths outside → `WallTheYard` (the existing "yard" project from Fortify, done now);
    - soldiers lost ≥ half the squad, or foes slain < 1 → `ArmBetter` (raise forge orders for
      helm, breastplate and shield to `max(squad size, 3)` for 2 seasons via `boosts`) and
      `SecondSquad` (`new_squad` if adults ≥ 12 and only one squad);
    - ≥ 3 bled to death → `MoreMedicine` (thread and cloth boosts, a second doctor).
    Each lesson is applied once when learned and again whenever its work is lost (traps broken).
  - The new `Lesson` sense (appended in P0) fires for a season after a lesson is learned, and
    pushes Fortify +1.0 and Muster +0.8 (for `ArmBetter`/`SecondSquad`).
- **Steps:**
  1. Step 0: trace, find and fix Steady's gate · `overseer.rs: ov_defences` (likely), with a
     note in the commit · test `overseer::tests::the_gate_is_whole_by_the_first_siege` for each
     style but Reckless: `trial_game(seed, style, until = day 168)` has
     `o.did("linked")`. If the cause is time, the test asserts it for ≥ 4 of 5 seeds.
  2. Watch a siege · `lessons.rs: watch_siege`, `schedule.rs` (1 line), `annals.rs: note_death`
     (push the kill spot via `self.siege_watch`) · test: `force_siege`, kill two dwarves
     outside with `kill_for_test`, and the watch has `deaths_outside == 2`.
  3. Learn and act · `lessons.rs: learn, apply`, `overseer_hooks.rs` (1 line on siege end),
     `overseer.rs: Overseer.lessons`, `ov_alert` (patience reads `bar_sooner()`) · test
     `lessons::tests::a_siege_that_got_in_brings_traps_and_a_second_squad`: craft a watch with
     `foes_inside`, `soldiers_lost=2` and 14 adults. After `learn`, there's a planned trap within
     15 of base, `squads.len()==2`, and a note "After the siege, …".
  4. The Lesson sense · `overseer_hooks.rs: extra_senses` · test: the brain shows `Lesson`
     > 0 and Fortify odds up after a lesson.
- **Overseer & screen:** it *is* the overseer. Each lesson is a note in its own voice
  (`o.voiced`), e.g. "They came through the gate while the bridge was down. Never again:
  traps in the corridor, and the bridge goes up sooner." There's a chronicle line per lesson
  and a "Lessons" list on the overseer's brain screen (`ui.rs`, a few rows under the senses).
  Counter: `lessons`.
- **Measure:** Steady bridge-up share from 12% to ≥ 50%. Later-siege dwarves lost from 4.6 to
  < 3.5. Steady standing after year 3 ≥ 4/10. ≥ 1 lesson per fort that survived a siege.
- **Risks / open questions:**
  - Lessons can over-fortify: a fort traps itself in and its haulers die in its own corridor
    (dwarves "know where their traps are": `traps_tick` only fires on creatures, so this is
    fine).
  - A second squad takes workers off the economy. It's gated by adults ≥ 12.
- **Parallel work:** **Writes** new `lessons.rs`; `overseer.rs: Overseer (1 field), ov_alert
  (patience line), ov_defences (step 0 fix)`; `annals.rs: note_death (1 line)`; `schedule.rs`
  (1 line); `overseer_hooks.rs` (2 lines). **Reads** P0. **Hot spots:** overseer.rs: keep to
  `ov_defences` and the one `patience` line; P8 edits `plan/ov_labors/ov_orders/ov_fit` and P11
  works in its own file, so they merge. **Agent size:** one agent, 2 sessions.

---

### S06 Expeditions worth the risk  (effort: 1.5 sessions; risk: low)

The "Done when" bar (< 5% of deaths) is already met: 2.4% in o01-50. What's left is that four
forts lost a whole squad in one roll. So the item is about squads that come home beaten
instead of not at all (OUT-17), carry food (OUT-19) and are led well (OUT-20).

- **Design:**
  - **Stricter sending** (`ambitions.rs: ov_venture`): on top of today's ≥ 3 members, strength
    ≥ 8, a second squad of ≥ 2 at home and day ≥ YEAR/2, add:
    - every member holds a metal melee weapon and ≥ 1 armour piece (`self.kit_ready(id)` from
      P2; until P2 lands, a local check on `held`/`armor`);
    - the fort has ≥ 12 adults;
    - no siege within a season (`o.last_siege + SEASON < tick`);
    - no Wounded sense.
    Cautious overseers never go to a Beast lair.
  - **Odds shown:** `expeditions.rs: fn odds(&self, ids, dest) -> f64` is pulled out of
    `expedition_returns` and shown in the expedition picker (`ui.rs`/`app.rs`, where
    `destinations()` is listed): "Odds: 64%".
  - **A fight in rounds, with turning back (OUT-17):** `expedition_returns` becomes three
    rounds. Each round rolls `won_round = chance(p)`. A lost round wounds 1–2 members for real:
    `body.wound(part, 8..14, true)`, so they come home bleeding and bandaged, needing the
    doctor (this feeds S02), and kills a member with probability `danger × 0.04`. After a lost
    round, the squad turns back if its courage fails: a check on the members' mean
    `personality.anxiety` and their regard for the leader, `knob exp.nerve`=0.5. Going home
    early (`back` moved to now + half the remaining days) ends the expedition "beaten" with
    whoever is left. Win 2 of 3 to triumph. Death caps: never more than half the squad, except
    at a Beast lair.
  - **Field rations (OUT-19):** `send_expedition` takes up to `2 × members` Drink and Meal items
    from stock (removed; noted in `Expedition.rations: u32`, `#[serde(default)]`). Without
    rations the odds are ×0.85 and members come home hungry (hunger = HUNGRY).
  - **The tactician (OUT-20):** the member with the highest `skill_level(fight_xp) +
    personality.diligence/25` leads. The odds get `+0.03 × leader level`, the leader gets +100
    fight XP, and the report names them: "Led by Urist, who read the ground well."
- **Steps:**
  1. Stricter sending, and the odds pulled out · `ambitions.rs: ov_venture`,
     `expeditions.rs: odds` · test `expeditions::tests::no_expedition_from_a_weak_fort`
     (10 adults, an unarmed squad → `ov_venture` returns false and the log says why).
  2. Rounds and turning back · `expeditions.rs: expedition_returns` · test
     `expeditions::tests::a_losing_squad_comes_home_early_with_its_wounded`: seed the rng so
     round 1 is lost (danger 7, strength 6). The squad is home before `back`, at least one
     member has `bleed_rate() > 0` (P1 API, or `bleeding > 0` before P1), and ≥ half came home.
  3. Rations and the tactician · `expeditions.rs: send_expedition, Expedition` · test: rations
     are taken from stock, and the leader is named in the returning message.
- **Overseer & screen:** the overseer's note says why it didn't send ("Kept the squad home:
  half of them have no armour"). Spotlight "Turned back" with the leader's name. Chronicle: "The
  Iron Hammers turned back from the lair of Emberwing, carrying two wounded." Counters:
  `exp_sent`, `exp_turned_back`, `exp_triumph`.
- **Measure:** expedition deaths < 5% (keep). No non-lair expedition loses > half its members.
  ≥ 1 "turned back" story in a 50-fort batch.
- **Risks / open questions:** stricter sending means fewer expeditions and fewer triumph
  milestones (Warlord's Glory verdicts). Watch the Glory count by style.
- **Parallel work:** **Writes** `expeditions.rs: send_expedition, expedition_returns,
  Expedition, odds (new)`, `ambitions.rs: ov_venture`, the `ui.rs` expedition picker (1 line).
  **Reads** P0 counters. Optional: P2 `kit_ready` and P1 `bleed_rate`, each with a local
  fallback. **Hot spots:** none beyond one ui line. **Agent size:** one agent, 1–2 sessions.
  Wave 1.

---

### S05 Food and water a fort can rely on: wells and cisterns (ENG-18)  (effort: 1.5 sessions; risk: med)

The overseer never builds a well today: there's no `BuildingKind::Well` in `overseer.rs` or the
layouts. Wells exist (`water.rs: can_place_well, working_wells, well_stand`), dwarves drink from
them (`game.rs: find_drink`), and doctors carry well water to patients (`medicine.rs:
Treatment::Water`). This item makes the overseer use them.

- **Design:** new `src/game/cistern.rs`, with `ov_water(&mut self, o)` registered in
  `overseer_hooks.rs: daily`. Projects are recorded in `o.done` (`"well"`, `"cistern"`).
  - **Well over found water.** Look for a tile inside the shelter (a burrow tile or a room
    tile on the main level, `!world.outside`) where a 1×1 channel down reaches water within
    `WELL_DEPTH`: an aquifer, an underground lake, or a cavern pool. That's a column search of
    `world` under each candidate. Cache the result on `dig_version` (`#[serde(skip)]
    water_scan: Option<(u64, Option<Pos>)>`): it's cheap and runs once a day. Designate a
    channel with the existing `Desig::Channel` and `place_building(Well, p)`. Note: "Sank a well
    by the dining hall: water from below, even in a siege."
  - **Cistern when there's no water below.** If the site has a river or pond on the surface
    (`world` tiles with `water > 0` and `outside`), plan a cistern:
    - a 3×5 chamber one level below the main level, under the dining hall;
    - a well above its middle;
    - a feed tunnel from the chamber towards the nearest surface water, under rock, with a
      floodgate (`BuildingKind::Floodgate`, already lever-worked: `traps.rs: pull_lever`) at
      the cistern end, and a lever in the dining hall.
    The overseer opens the gate (pull) when the cistern is low (`well_has_water` false) and
    shuts it when the chamber's tiles reach `water >= 4`. A siege shuts it too, so goblins
    can't swim in (`Mover::Goblin` already can't pass a shut floodgate).
    The plan respects the existing safety rules from S05 step 3: no hall beside or under water,
    and the feed tunnel stops a tile short of the water. The breakthrough tile is dug only after
    the floodgate is built.
  - **Fallback:** forts with neither get no well. The trouble `Lack::Drink` already covers
    barrels.
  - **Sieges:** sheltering dwarves already drink from wells in the burrow, since `find_drink`
    accepts wells.
- **Steps:**
  1. A well over found water · `cistern.rs: find_well_spot, ov_water (well arm)`,
     `overseer_hooks.rs` (1 line) · test `cistern::tests::the_overseer_sinks_a_well_over_water`:
     a Game with a pocket of water under the main level (as `water.rs: tests::dig_shaft` does).
     After ov_setup and 3 days of `step()`, `working_wells()` is non-empty.
  2. A cistern fed through a floodgate · `cistern.rs` (cistern arm, gate control in an
     `ov_alert_hooks` line) · test `cistern::tests::a_cistern_fills_and_its_gate_shuts`: carve
     the chamber and feed by hand (like `water.rs: tests::tunnel`), let the overseer run, see
     `water >= 4` in the chamber, the gate shut, and the well working.
  3. Thirst in sieges · test `cistern::tests::sheltering_dwarves_drink_from_the_well`: civ
     alert on, every barrel removed, a working well inside the burrow. Nobody's thirst passes
     THIRSTY + 2000 over two days.
- **Overseer & screen:** the notes above. The trouble `Lack::Drink` gets a fix arm: "dig a
  well" when `ov_water` has a spot. Chronicle line "A well was sunk in X". Counters: `wells`,
  `cisterns`, `well_drinks` (count in `find_drink` when a well is chosen).
- **Measure:** wells in ≥ 30% of non-Reckless forts. Sane thirst deaths stay 0. No new
  drowning deaths (o01-50: 7). Watch the drowned count closely; a cistern that floods the
  halls is the main risk.
- **Risks / open questions:**
  - Flooding. The breakthrough-last rule and a shut gate on any siege are the guards. If
    drownings rise, ship step 1 and hold step 2.
  - `Desig::Channel` exists (`world.rs`, handled in `game.rs` ~737), and so does
    `BuildingKind::Floodgate`, so no new dig type is needed. `water.rs: tests::dig_shaft` shows
    the tiles a working well needs.
  - Decision for the user: should the overseer also drain flooded halls (pumps), or leave that
    to W26?
- **Parallel work:** **Writes** new `cistern.rs`; `overseer_hooks.rs` (2 lines);
  `troubles.rs: fix` (one arm: Lack::Drink). **Reads** P0. **Hot spots:** troubles.rs `fix`
  (P8 adds an arm for Lack::Doctor; different arms, so a trivial merge). **Agent size:** one
  agent, 1–2 sessions. Wave 1.

---

### S07 Dwarves who fight back  (effort: 4.5 sessions; risk: med)

Covers MIL-01 and MIL-02 (P2, wave 1), MIL-05 and the openings half of MIL-04 (P9, wave 2), and
MIL-03 (P11, wave 2). The reshape half of MIL-04 is S02's P1.

- **Design:**
  - **Metal crossbows and bolts (MIL-01).** `buildings.rs: Recipe` gets `ForgeCrossbow` and
    `ForgeBolts` **appended at the end** of the enum. Inputs: `[Bar, Fuel]`. Outputs: one
    Crossbow of the bar's metal, or 3 Bolt bundles of the bar's metal. Both go in the Forge
    and MagmaForge recipe lists, with a name and a key.
    - `Dwarf` gets `#[serde(default)] pub ammo_mat: Option<Mat>`, the best bolts in the quiver.
      `do_equip` sets it when a bundle is taken: `max` by `bolt_damage`.
    - `military.rs: try_shoot` damage becomes `bolt_damage(ammo_mat)`: Wood/None 9, Bone 9.5,
      Copper 10, Bronze 11, Iron 12, Steel 14 (knob `bolt.mult`). It's edged as today. The
      crossbow's own material adds +0.5 skill for metal (a better draw).
    - `UniformWeapon::Best` already refuses crossbows, so a marksdwarf never clubs while melee
      weapons are in stock (`ov_army` already switches the squad to Best when melee ≥ squad).
      For a squad that is on crossbows, `do_attack` prefers to back off (P9).
    - `ov_orders`: with a forge, `ForgeBolts` 6 and `ForgeCrossbow` 2 (Warlord: 4).
      `WoodCrossbow`/`Bolts` drop to 1/2 once a forge is built (wood stays for practice:
      `drill_xp` already uses them).
  - **Kit on before the first siege (MIL-02).** New `military.rs: kit(&self, id) -> Kit`:
    ```rust
    pub struct Kit { pub weapon: Slot3, pub helm: Slot3, pub mail: Slot3, pub shield: Slot3 }
    pub enum Slot3 { Worn, InStock, None }   // InStock = a free fitting item exists
    pub fn kit_ready(&self, id: u32) -> bool  // weapon Worn (metal) && ≥1 armour Worn
    ```
    - First, **why 10–32 pieces lay unworn** (the WISHLIST trace): `equip_soldier` runs only
      from `choose_military` when the soldier is idle, but a soldier on `Station` re-chooses
      only when far from the station (`squad_task`), and `Train` loops. So `go_train` and the
      `Station` arm get a cheap `if tick % 300 == id % 300 && self.equip_soldier(d) { return }`.
      `find_item` may also skip items in bins (`it.container`); check `find_item`'s filter and
      allow equipment out of bins.
    - Overseer: `readiness.rs` (new), `ov_readiness(o)` in the daily hooks. For every soldier
      with an `InStock` slot it forces `equip_soldier`. For a `None` slot it raises
      `Lack::Kit(Slot)` (appended to `Lack`), and `fix` boosts the forge orders (or leather at
      no forge). The sense `Unarmed` (appended in P0) is the share of soldiers not
      `kit_ready`. It feeds Muster +1.2 and blocks Venture.
    - When the scouts' warning comes (`maybe_siege` sets `incoming`), the readiness check runs
      at once, so kit is on before the army arrives.
  - **Archers who act like archers (MIL-05; P9).**
    - In `combat.rs: do_attack`: a soldier with `holds_crossbow && ammo > 0` keeps 3–6 tiles
      off. If the target is adjacent and a walkable neighbour increases distance (the same
      scoring as `do_flee`), step there instead of swinging. They melee only when cornered
      (no better tile) or out of ammo. If out of ammo and bolts are in stock, the soldier goes
      to fetch them (`wants_bolts` → Equip) when no threat is within 5.
    - Squad 2 (from S04's `SecondSquad` lesson, or Muster) gets the crossbow uniform once there
      are ≥ 2 metal crossbows, and its station is `base.offset(5..7, 0..1, 0)`, behind the
      drawbridge with a clear shot to the gate tiles (`clear_shot`). `ov_alert` picks its
      station with `clear_shot(st, base.offset(1,0,0))`.
    - Enemy archers: none exist yet (goblins carry melee only). Skip for now and note it in
      A-milestone items.
  - **Openings: a downed foe is finished (MIL-04 second half; P9).** In `combat.rs: strike`,
    if the defender's body is `unconscious()`, the hit chance is 0.95 and the part is forced to
    Head or UpperBody, with `+50%` damage. That's one `if` before `pick_part`. It applies both
    ways, so goblins finish downed dwarves too. To leave time for rescue, a siege creature with
    an unconscious target re-targets to the nearest *standing* dwarf it can reach, and finishes
    the downed one only when nobody standing is within 6. Soldiers (`combat_awareness`) pick
    downed foes first when adjacent ("finishes off the wounded goblin"). Wounded goblins that
    break (40% blood) still flee, but a soldier adjacent to a breaking goblin gets one free
    swing (an opening as it turns): one `dwarf_attack` call when `leaving` flips.
  - **The overseer hunts predators (MIL-03; P11).** New `src/game/hunts.rs`, `ov_hunts(o)`
    in `overseer_hooks.rs: alert` (every 10 ticks, cheap). A prowler is an
    `alive && hostile() && !siege && !legendary() && !tame` creature with `aggro > 0` within
    30 of top or embark, on the surface or near the halls. From P3 this becomes
    `self.prowlers()`; until then a local filter. If there's no siege and no `Kill` order,
    pick the squad with the most `kit_ready` members, at least 2 and not away on an
    expedition, and `order_kill(squad, nearest prowler)`. After a kill, `creature_died` resets
    the order to Train, and the next check assigns the next pack member.
    - Reckless never hunts. Cautious hunts only with ≥ 3 ready soldiers. Bold hunts with 2,
      and anything that isn't a megabeast.
    - Meanwhile outdoor civilians within 20 of a prowler stop their outdoor jobs: that's
      `Flee` if within 7 (as today). For 7–20 the overseer sets a short `outdoor_ban_until =
      tick + 300` (`#[serde(default)]` on Game), and `find_dig(woodcutting)`, `find_gather`,
      `find_fish` and `find_farm` check it for outside tiles. Those finders sit in
      `finders.rs`, so the guard is one `if` at the top of the finder table entries for
      outdoor work.
    - The `Prowlers` sense (appended in P0) fires while a pack is near and the squad is weak.
      It feeds Muster +0.8 and Fortify +0.4.
- **Steps:**
  1. **P2-a** Metal crossbows and bolts · `buildings.rs: Recipe (append 2), inputs, produce,
     name, key, Forge recipe list`; `dwarf.rs: ammo_mat`; `combat.rs: do_equip` (bolt arm);
     `military.rs: try_shoot, bolt_damage`; `overseer.rs: ov_orders` (3 lines) · tests:
     `industry_tests.rs: forge_makes_iron_bolts_and_crossbows`, and
     `military::tests::iron_bolts_hit_harder`, which uses `strike` with a fixed rng seed
     (iron bolts' mean damage > wood + 2).
  2. **P2-b** Kit on before the first siege · new `readiness.rs` (kit, kit_ready,
     ov_readiness), `combat.rs: equip_soldier` callers (`go_train`, Station arm), `troubles.rs:
     Lack::Kit`, `fix` arm, `overseer_hooks.rs` (1 line), `combat.rs: maybe_siege` (1 call) ·
     test `readiness::tests::kit_is_on_before_the_first_siege`: a squad of 3 stationed, iron
     axes, helms and breastplates in a bin in a stockpile. `force_siege` (the warning) and 300
     ticks later all three wear helm and mail and hold axes.
  3. **P9-a** Openings · `combat.rs: strike` (1 `if`), `creature_think` siege re-targeting,
     `creature_tick` (the parting swing), `combat_awareness` (downed first) · test
     `combat::tests::a_downed_goblin_is_finished`, and arena rerun: kits that win should win
     in fewer ticks. `combat::tests::goblins_leave_the_fallen_for_the_standing`.
  4. **P9-b** Archers keep their distance · `combat.rs: do_attack` (crossbow arm),
     `overseer.rs: ov_alert` (squad-2 station, 3 lines), `ov_army` (squad-2 uniform) · test
     `military::tests::a_marksdwarf_backs_off_and_shoots`: a goblin adjacent, open ground, 10
     bolts. After 100 ticks the marksdwarf has shot ≥ 3 times and never meleed while a retreat
     tile existed.
  5. **P11** The overseer hunts predators · new `hunts.rs`, `overseer_hooks.rs` (1 line),
     `game.rs` (`outdoor_ban_until` field), `finders.rs` (outdoor guard) · test
     `hunts::tests::the_overseer_sends_a_squad_after_wolves`: an overseer fort with a ready
     squad of 3 and two wolves spawned 15 tiles from the embark. Within 100 ticks the squad
     has a Kill order, within a day both wolves are dead, and the note says "Sent … after the
     wolves". Also `hunts::tests::reckless_never_hunts`.
  6. **"Done when" check** · test `combat::tests::soldiers_in_metal_kill_their_goblins`
     (WISHLIST "Done when"): 4 soldiers with iron axes, helms, mail and shields at a station
     in the gate corridor, `force_siege` with 4 goblins, bridge down. After a day the goblins
     slain ≥ 4 (≥ 1 per soldier) in ≥ 7 of 10 seeds.
- **Overseer & screen:** readiness shows on the squads panel as a kit grid (one row per
  soldier: W H M S, coloured worn/in stock/none), the WISHLIST "equipment grid". Hunts are
  announced ("The Iron Hammers set out after the wolves.") and a hunt's end gets a chronicle
  line ("The Iron Hammers killed three wolves that had been taking the woodcutters."). The
  overseer notes for kit fixes. Counters: `hunts`, `hunt_kills`, `kit_shortfalls`, `bolts_metal`.
- **Measure:**
  - goblins slain per siege from 1.2 to ≥ 2.5; bolts shot per siege from 3.9 to ≥ 10;
  - siege blows: "foes on soldiers" share drawing blood stays ≤ 20%;
  - wolves' share of deaths from 18% to < 8% (with P3);
  - "bled to death" < 25% (with S02).
- **Risks / open questions:**
  - Openings speed up dwarf deaths too. Goblins re-target the standing first, but in a lost
    fight it still ends faster. Watch the first-siege losses (2.3, must stay < 3).
  - Hunting sends soldiers out of the gate. A siege arriving during a hunt finds them outside.
    `ov_alert` already re-stations on danger; make the kill order drop when `incoming` is set.
  - Decision for the user: should the overseer also go after *legendary* beasts (megabeasts)?
    Recommend no (they're boss fights for a player to choose).
- **Parallel work:**
  - **P2:** **Writes** `buildings.rs: Recipe` and its matches (append), `items.rs` (none),
    `dwarf.rs: ammo_mat`, `combat.rs: do_equip, equip_soldier callers`, `military.rs:
    try_shoot`, new `readiness.rs`, `troubles.rs: Lack (append), fix (arm)`,
    `overseer.rs: ov_orders (lines)`. **Reads** P0. **Hot spots:** buildings.rs (Recipe enum:
    append-only), troubles.rs (append). **Agent size:** one agent, 2 sessions. Wave 1.
  - **P9:** **Writes** `combat.rs: strike, do_attack, combat_awareness, creature_think (siege
    targeting), creature_tick (parting swing)`, `overseer.rs: ov_alert (station), ov_army
    (uniform)`. **Reads** P1 (unconscious under new blood), P2 (crossbows, `kit_ready`).
    **Hot spots:** combat.rs is shared with P5's guarded call and P3's wildlife branch, which are
    different functions or branches by then (P3 moves the wildlife branch out). **Agent size:**
    one agent, 2 sessions. Wave 2.
  - **P11:** **Writes** new `hunts.rs`, `overseer_hooks.rs` (1 line), `Game` (1 field),
    `finders.rs` (guard). **Reads** P3 `prowlers()` (C4), P2 `kit_ready()` (C3). **Agent
    size:** one session. Wave 2.

---

### S08 Wild things with sense  (effort: 5 sessions; risk: med)

Covers CRE-01, CRE-03 and CRE-15 (P3, wave 1), and CRE-02, CRE-22, CRE-23 and CRE-29 (P14,
wave 3). Wolves killed 190 dwarves in o01-50 (180 bled, 162 under the sky, 1 soldier). Bears
killed 43.

- **Design:**
  - **First, move wildlife out of combat.rs:** the non-siege, non-special branch of
    `combat.rs: creature_think` (the `if c.target.is_none() { … prey … }` tail) and
    `combat.rs: wildlife` move to a new `src/game/wildlife.rs` as `predator_think(c)` and
    `wildlife()` (no behaviour change, its own commit). `creature_tick`'s "lose nerve" block
    calls `self.wild_nerve(c)` there too.
  - **Predator discretion (CRE-01).** `predator_think` scores each dwarf within `aggro + 6`:
    `odds = prey_weakness / (1 + escorts)`, where
    - escorts are other dwarves within 4 of the prey (soldiers count 3);
    - prey weakness is 1.0 base, +0.5 if bleeding or limping, +0.5 if a child, −0.5 if they
      hold a weapon;
    - pack strength is the number of pack members within 8 (Creature gets
      `#[serde(default)] pub pack: u32`, shared by a spawn group: the id of the first spawned).
    It attacks if `odds × pack_strength >= knob wild.nerve` (1.5). Otherwise it **stalks**.
  - **Stalking (CRE-15).** Creature gets `#[serde(default)] pub stalk: Option<u32>`, the prey it
    follows. It keeps `aggro + 3 .. aggro + 6` tiles off (a path to a tile at that range, once
    every `think_cd`). It strikes when the prey's escorts drop to 0 or at night: `tick % DAY`
    in the last quarter of the day. Weather's light, if present, could be used instead; check
    `weather.rs`. Switching prey: when a weaker dwarf comes into range. It gives up after a
    day (`leave_tick` as today).
  - **Wary once wounded (CRE-03).** Today a non-siege creature leaves when `badly_hurt()`
    (blood < 60% or a part ≤ 0). New: on the first wound of ≥ 25% of a part, a wild creature
    breaks off for 200 ticks (`#[serde(default)] pub wary_until: u64`). It returns only to
    stalk. When a pack member dies, the pack breaks: all `pack` mates within 12 set `leaving`
    (wolves routing, like `rout()` for goblins).
  - **Fire and intimidation (CRE-02; P14).** A dwarf attacked with ≥ 2 other non-child dwarves
    within 3 "stands together". The attacker rolls `chance(0.3 × (allies-1))` each swing to
    break off and get `wary_until`. Creatures within 2 of `self.flames` or a fire tile flee,
    and get `#[serde(default)] pub fears_fire: bool` after being burned (`fire.rs` damage
    sites). Fire-fearing creatures keep 6 tiles from burning tiles.
  - **Silent woods (CRE-22; P14).** When a creature starts stalking, outdoor dwarves within 15
    get a one-time `warn` ("The birds have gone quiet near the woodcutter Urist.") and a
    thought (+3, unease). With P11, the overseer reads it as an early prowler alert:
    `prowlers()` includes stalkers a day earlier.
  - **Wildlife from lairs (CRE-23; P14).**
    ```rust
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Lair { pub pos: Pos, pub species: Species, pub pop: u32, pub cap: u32, pub found: bool }
    ```
    `Game` gets `#[serde(default)] pub lairs: Vec<Lair>`, made at embark from `fauna()`: 1–2 pack
    lairs and 1 rare lair at the map edge, in woods or hills. `wildlife()` spawns a pack *from*
    a lair with `pop > 0` (taking n from pop). Leaving creatures path back to their lair, not
    to any edge, and return to `pop`. A creature followed home marks `found`, and the overseer
    (P11) can kill-order the lair: soldiers go to it, and each kill there drops `pop`.
  - **Living populations (CRE-29; P14).** Lair `pop` regrows by 1 per season up to `cap`.
    Hunting (the soldiers' `hunt` flag) and kills take from `pop`. Prey populations (deer,
    elk) get their own lairs; with `pop == 0` for prey, predators come closer to the fort
    (desperation: `wild.nerve × 0.7`).
- **Steps:**
  1. **P3-a** Move wildlife to its own file (refactor) · `combat.rs` → `wildlife.rs` · tests:
     existing `each_land_has_its_own_wildlife` and `year_with_wildlife_and_caverns_runs` pass
     unchanged.
  2. **P3-b** Packs and discretion · `wildlife.rs: predator_think`, `creature.rs` (`pack`) ·
     test `wildlife::tests::wolves_pass_a_squad_and_take_the_straggler`: 3 soldiers together
     and one woodcutter 15 tiles off, two wolves spawned between. After 300 ticks no wolf has
     attacked a soldier, and the woodcutter is stalked or attacked. That's the WISHLIST
     "Done when" picture.
  3. **P3-c** Stalking and the nerve to break · `wildlife.rs`, `creature.rs` (`stalk`,
     `wary_until`), `combat.rs: creature_tick` (one call `wild_nerve`) · tests
     `wildlife::tests::a_wounded_wolf_breaks_off` and `the_pack_scatters_when_one_dies`.
  4. **P14-a** Standing together and fear of fire · `wildlife.rs`, `combat.rs:
     creature_attack` (1 call), `fire.rs` (set `fears_fire`) · test
     `wildlife::tests::three_dwarves_together_drive_off_a_wolf`.
  5. **P14-b** Silent woods · `wildlife.rs` · test: a stalker near an outdoor dwarf → one log
     line "gone quiet".
  6. **P14-c** Lairs and living populations · `wildlife.rs: Lair`, `Game` (1 field),
     `game.rs: Game::new` (make lairs, 1 call), `schedule.rs` (seasonal regrowth, 1 line),
     `hunts.rs` (lair kill order) · tests `wildlife::tests::packs_come_from_and_return_to_their_lair`
     and `a_hunted_out_lair_sends_nothing`.
- **Overseer & screen:** `prowlers()` (C4) feeds P11's hunts and the `Prowlers` sense. Lairs
  that are `found` show as a map marker (`ui.rs` map overlay, glyph `Ω` dim) and as hunt
  targets. Messages: "A wolf shadows Urist at the treeline.", "The wolves scatter!",
  "Tracked the wolves to their den north of the river." Chronicle: "the wolves of the north
  den were hunted out". Counters: `stalks`, `packs_broken`, `lair_kills`.
- **Measure:** wildlife deaths (wolf + bear + lion + jaguar + alligator + scorpion + viper +
  polar bear) halve per fort: o01-50 has about 233 surface-wildlife deaths in 48 forts, so the
  target is < 2.4 per fort. Soldiers' deaths to wildlife stay small (no berserker charges into
  squads).
- **Risks / open questions:**
  - Smarter predators may kill *more* lone woodcutters. That's why S08 ships with P11's hunts
    and the outdoor ban in the same wave-3 batch.
  - Night detection: check whether `weather.rs` has light; otherwise use a quarter-day window.
  - Performance: stalking re-paths every `think_cd` (15 ticks). Use `can_reach`'s cache and
    path only to a ring tile with `path::search` limit 900, as `creature_wander` does.
- **Parallel work:**
  - **P3:** **Writes** new `wildlife.rs` (moved and new code), `combat.rs: creature_think`
    (wildlife tail replaced by one call), `combat.rs: wildlife` (moved), `creature_tick`
    (one call), `creature.rs: Creature` (3 fields). **Reads** P0. **Hot spots:** combat.rs. P5
    edits the *siege* branch and P9 (wave 2) edits other functions, so tell each agent to keep
    to its functions. **Agent size:** one agent, 2 sessions. Wave 1.
  - **P14:** **Writes** `wildlife.rs`, `fire.rs` (1 line), `Game` (1 field), `schedule.rs`
    (1 line), `hunts.rs` (lair target). **Reads** P3, P11. **Agent size:** 2–3 sessions; split
    P14-a+b (1 session) and P14-c (2 sessions). Wave 3.

---

### S09 Madness that mends  (effort: 2 sessions; risk: low)

Covers MND-06 and MND-31. 20 of 21 thirst deaths in o01-50 were mad dwarves, and 40 mad
dwarves died in all.

- **Design:** new `src/game/breakdowns.rs`. `Dwarf` gets
  `#[serde(default)] pub fit: Fit`, with
  `enum Fit { #[default] None, Berserk { until: u64 }, Catatonic { since: u64 } }`.
  `insane`/`melancholy` stay as they are: melancholy is `insane && melancholy`, and raving is
  `insane && !melancholy`.
  - **Mad dwarves still drink (the fix for the thirst deaths).** The `sipping` rule in
    `game.rs: update_dwarf` drops `&& d.melancholy`: a raving dwarf who is desperate
    (`thirst > THIRSTY + 6000`) drinks from the nearest water or barrel, grabbing a drink with
    no seat. That's a one-token change, its own commit, with a test.
  - **Shifting breakdowns (MND-06).**
    - Raving can ease: from `mad_since + 10 days`, a daily `chance(0.06)` (×2 if loved:
      the same `loved` test as melancholy) to turn into melancholy ("the rage burns out
      into grief"). From there the existing lifting applies.
    - Melancholy with stress still ≥ 650 after 10 days has a daily `chance(0.03)` to turn
      raving.
  - **Berserk (MND-06).** In `update_dwarf`'s tantrum trigger (`stress >= tantrum_threshold()`),
    a dwarf with `personality.temper > 30` who already tantrumed within 10 days goes berserk
    instead (`Fit::Berserk { until: tick + 600 }`). That's a hostile dwarf: `Task::Berserk`
    walks to the nearest dwarf and strikes with `dwarf_fighter` (its held weapon). Report
    `CrimeKind::Assault`.
    - Soldiers and the sheriff treat a berserk dwarf as a threat: `threats_near` gains berserk
      dwarves through a small `berserk_near(p, r)` used by `combat_awareness`, and soldiers get
      `Task::Attack`-like "subdue" (blunt blows only: `edged=false`, damage halved, stopping at
      `unconscious()`). Civilians flee them.
    - It ends when `until` passes, when knocked unconscious (subdued → jailed if there's a
      jail: `justice.rs`), or in death. Afterwards: stress −300, then 50% back to sane, 30%
      melancholy, 20% raving.
  - **Catatonia (MND-31).** A rare outcome where misery would make melancholy: 15% chance,
    more for `anxiety > 25`, is `Fit::Catatonic` instead (`insane=true`, `Task::Catatonic`:
    lies where they are). Catatonic dwarves count as "lying" for `medicine.rs: find_patient`
    (Water and P8's Feed), so they don't die of thirst if a well or drink exists.
    - A friend (regard ≥ 30) or spouse, idle, gets the finder `find_coax`
      (`Task::Coax { patient }`: sits beside them 200 ticks), with a daily
      `chance(0.25 per coax)` to bring them back ("Urist sat with Domas until they spoke
      again.").
- **Steps:**
  1. Mad dwarves drink · `game.rs: update_dwarf` (1 token) · test
     `psyche_tests::a_raving_dwarf_still_drinks_when_desperate`: a raving dwarf with a barrel
     of drink 10 tiles away and `thirst = THIRSTY + 7000` is alive and has drunk after 600 ticks.
  2. Breakdowns shift · `breakdowns.rs: shift_daily`, `schedule.rs` (1 line) · test: a raving
     dwarf with a living spouse, after 20 days with a fixed-seed rng, is melancholic or sane.
  3. Berserk and subdued · `breakdowns.rs`, `game.rs: update_dwarf` (tantrum branch, 3 lines),
     `combat.rs: combat_awareness` (1 call), `do_task` (Berserk arm) · test
     `breakdowns::tests::a_berserk_dwarf_is_subdued`: a berserk dwarf, 2 soldiers near. Within
     400 ticks they're unconscious or calm, and nobody is dead. That's the WISHLIST "Done when".
  4. Catatonia and coaxing · `breakdowns.rs: find_coax, do_coax`, `finders.rs` (1 row),
     `medicine.rs: find_patient` (lying includes catatonic) · test: a catatonic dwarf with a
     friend nearby recovers within 5 days, and a doctor brings water meanwhile.
- **Overseer & screen:** the overseer treats a berserk dwarf like a threat: `ov_alert` stations
  the squad near if one rages for > 100 ticks. Sentimental overseers assign a coaxer
  (`ov_daily` hook: set the friend's `idle` priority). Spotlights for "berserk!" (red) and
  "spoke again" (blue). The Health/psyche view shows the kind of breakdown. Counters:
  `berserk`, `catatonic`, `mended` (any breakdown that ended sane).
- **Measure:** mad thirst deaths 0 (o01-50: 20). ≥ 1 "mended" per 10 forts. Berserk kills
  ≤ 0.2 per fort (it must not become a new killer).
- **Risks / open questions:**
  - Berserk dwarves holding axes could kill. Soldiers subdue with blunt blows, but a civilian
    who's struck fights back (`struck_recently`) at full damage. Decision for the user: may a
    berserk dwarf kill? Recommend yes, rarely. It's a story ("Urist slew her friend in a
    rage").
  - Catatonic dwarves need someone to care. In Reckless forts, with no hospital and no doctor
    (P8 changes that for the others), they may still die. That's acceptable.
- **Parallel work:** **Writes** new `breakdowns.rs`; `dwarf.rs: fit` (1 field); `game.rs:
  update_dwarf` (sipping token and a 3-line tantrum branch), `do_task` (2 arms);
  `combat.rs: combat_awareness` (1 call); `medicine.rs: find_patient` (1 condition); `finders.rs`
  (1 row); `schedule.rs` (1 line). **Reads** P0 (Task variants), and P8's Feed treatment if
  present. **Hot spots:** update_dwarf (P7 also adds lines there: different places, but
  serialize the merge of P4 before P7). **Agent size:** one agent, 2 sessions. Wave 1.

---

### S10 Knobs for the batch  (effort: 1.5 sessions; risk: low)

Covers OTH-04 (in P0, wave 0) and OTH-06 (P15, wave 3).

- **Design:**
  - **Tunable thresholds (OTH-04):** see P0. Then wire the existing siege, megabeast, thief and
    wildlife triggers to knobs, in their own files, as part of P15 or by each item's agent:
    - `combat.rs: maybe_siege` uses `siege.base`=0.3, `siege.per_pop`=0.02 and
      `siege.wealth_div`=200000;
    - `siege_strength` uses `siege.first_min`=2, `siege.first_max`=3, `siege.cap`=20 and
      `siege.per_broken`=1;
    - `wildlife` uses `wildlife.chance`=0.18; `maybe_megabeast` uses `beast.base`=0.12;
      `maybe_thief` uses `thief.chance`=0.25.
  - **Path throttling (OTH-06):** a profile comes first. `TRIAL_PROF` and `SPIKE_MS` exist
    (`game.rs: prof_add`). Add a counter of failed path searches per caller (`count("pathfail.<site>")`,
    off unless `TRIAL_PROF`). Expected hot spots:
    - `combat.rs: creature_tick` `astar(…, 8000)` towards a target, which already backs off 90
      ticks on failure;
    - `creature_think` siege `path::search(SEARCH_LIMIT)` every 15 ticks for creatures shut out
      (it backs off 120 only when shut out);
    - `choose_task` finders failing every idle cycle (`idle_cd` 20–40 ticks).
    Creature gets `#[serde(default)] pub path_fails: u8`. Each failure doubles the wait
    (`think_cd = 15 << path_fails.min(4)`), and a success or a `dig_version` change resets it.
    For dwarves, a per-finder failure memory `#[serde(skip)] finder_fail: HashMap<(u32 dwarf,
    u8 finder), (u64 tick, u64 dig_version)>` on Game skips a finder that failed for that
    dwarf within `40 << n` ticks with the same `dig_version`. It lives in `finders.rs` and wraps
    the table walk.
- **Steps:**
  1. (P0 step 2) Knobs and counters, as above.
  2. **P15-a** Siege and wildlife triggers on knobs · `combat.rs`/`wildlife.rs` (constants →
     `self.knob`) · test `knobs::tests::siege_size_cap_is_a_knob` (set `siege.cap=4` and see
     `siege_strength() <= 4`). This is the WISHLIST "Done when": "A batch can be rerun with a
     changed threshold from the command line."
  3. **P15-b** Path throttling · `creature.rs` (`path_fails`), `combat.rs: creature_tick,
     creature_think` (backoff), `finders.rs` (finder memory) · tests
     `path_throttle::a_creature_shut_out_searches_less` (count `path::search` calls through a
     `#[cfg(test)]` counter, sealed room, ≤ 1/4 of today's in 1000 ticks) and the
     `schedule.rs` golden-story test, which may change only within tolerance. If the story
     moves, record a new golden list and say why in the commit.
  4. **P15-c** The tuning pass (S02 step 8): knob sweeps for bleed, siege and wildlife.
     Defaults are baked in from the best run, recorded in WISHLIST "Result".
- **Overseer & screen:** none. The batch report header lists the knobs set.
- **Measure:** `overseer_speed` / TRIAL_PROF: ms per fort-year down ≥ 15% on the 5-fort
  profile, with the same verdicts.
- **Risks / open questions:** the finder memory can make a dwarf miss work that just appeared
  without a dig (a new item to haul). Key it on `dig_version` *and* a cheap `items_version`
  bumped in `spawn_item`. Ask before adding `items_version`, if it isn't already there.
- **Parallel work:** P0 (wave 0) as above. **P15:** **Writes** `combat.rs`/`wildlife.rs`
  constants, `creature.rs` (1 field), `finders.rs` (wrapper). **Reads** everything (it tunes).
  **Agent size:** one agent, 2 sessions. Wave 3, last.

---

## Interface contracts (fixed before wave 1 starts)

These go in during P0 or at the start of the named package, with stub bodies if needed, so
dependents can code against them.

**C1 — Body bleeding (body.rs, P1).**
```rust
impl Body {
    pub fn bleed_rate(&self) -> i32;                 // tenths of blood per 10 ticks, all parts
    pub fn worst_bleed(&self) -> Option<Part>;       // part with the highest rate
    pub fn ticks_to_death(&self) -> Option<u64>;     // None if not bleeding or it will clot first (still)
    pub fn press(&mut self, p: Part);                // set pressed[p]
    pub fn bandage(&mut self, p: Part);              // bandaged[p] = true, bleeds[p] /= 2
    pub fn suture(&mut self);                        // bleeds = 0, pressed = bandaged = false
    pub fn tick_with(&mut self, p: &BleedParams, moving: bool);
}
#[derive(Clone, Copy, Debug)] pub struct BleedParams { pub cut: i32, pub gash: i32, pub mangle: i32,
    pub sever: i32, pub vital_pct: i32, pub clot_still: i32, pub clot_moving: i32 }
impl Default for BleedParams { /* 3, 8, 12, 45, 150, 6, 2 */ }
impl Game { pub(crate) fn bleed_params(&self) -> BleedParams; }   // from knobs, cached
```
Serde: `bleeds`, `pressed`, `bandaged` and `blood_frac` default to zeros. `bleeding` is kept
and stays in sync.

**C2 — Knobs and counters (knobs.rs and annals.rs, P0).**
```rust
impl Game { pub fn knob(&self, name: &'static str, default: f64) -> f64;
            pub(crate) fn count(&mut self, key: &str, n: u32); }
pub struct Knobs { pub set: BTreeMap<String, f64> }      // Game: #[serde(default)] knobs
// Annals: #[serde(default)] counters: BTreeMap<String,u32>; Trial: counters, knobs (serde default)
pub fn run_trial(seed: u64, style: Style, years: u64, knobs: &Knobs) -> Trial;
```

**C3 — Soldier kit (readiness.rs, P2).**
```rust
pub enum Slot3 { Worn, InStock, None }
pub struct Kit { pub weapon: Slot3, pub helm: Slot3, pub mail: Slot3, pub shield: Slot3 }
impl Game { pub fn kit(&self, dwarf: u32) -> Kit; pub fn kit_ready(&self, dwarf: u32) -> bool;
            pub(super) fn bolt_damage(mat: Option<Mat>) -> f32; }
```

**C4 — Prowlers (wildlife.rs, P3).**
```rust
impl Game { /// Wild hostiles near the fort (not sieges, not legends), nearest first,
            /// stalkers included: (creature id, pos, pack id).
            pub fn prowlers(&self, near: Pos, r: i32) -> Vec<(u32, Pos, u32)>; }
// Creature: #[serde(default)] pack: u32, stalk: Option<u32>, wary_until: u64
```

**C5 — Overseer hooks (overseer_hooks.rs, P0).**
```rust
impl Game {
    pub(super) fn ov_daily_hooks(&mut self, o: &mut Overseer);   // one line per item
    pub(super) fn ov_alert_hooks(&mut self, o: &mut Overseer);   // one line per item; cheap
    pub(super) fn extra_senses(&self, o: &Overseer) -> Vec<(Sense, f64, Vec<(Ambition, f64)>)>;
}
// Sense gains (P0): Lesson, Prowlers, Unarmed, Wounded
```

**C6 — Finder table (finders.rs, P0).**
```rust
pub(super) type Finder = fn(&mut Game, &mut Dwarf) -> bool;
pub(super) const FINDERS: &[(&str, Finder)];   // order = priority; items insert rows
```

**C7 — Task variants (dwarf.rs, P0):** `FirstAid { patient, item: Option<u32> }`,
`Rescue { patient, bed: Option<u32> }`, `Press`, `Catatonic`, `Berserk`, `Coax { patient }`,
appended at the end. `is_need()` includes `Press` and `Catatonic`.

**C8 — Siege end hook (annals.rs, P0):** `annals_tick`, when it closes a siege, calls
`self.on_siege_end(i)` (in `overseer_hooks.rs`), where P12 (lessons) and P5 (captives' grudges)
add lines.

**C9 — Captives (captives.rs, P5):** `Game.captives: Vec<Captive>` (serde default);
`fn take_captive(&mut self, c: &Creature, cage: u32)`; `Annals.grudges: BTreeMap<u32,u32>`
read by `siege_strength`.

## Hot-spot discipline (P0 does these before the waves)

1. `Game::step`'s periodic calls → `schedule.rs`, one line per system (C-free merges).
2. `choose_task`'s job finders → the `FINDERS` table (C6).
3. Overseer extension points → `overseer_hooks.rs` (C5, C8); `brain` gets one `extra_senses` call.
4. All new `Task` and `Sense` variants appended once, in P0 (C7, C5).
5. Report metrics through `counters` (C2), never new `Trial` fields per item.
6. Append-only rules for shared enums: `Recipe`, `Lack`, `StockFilter`, `Treatment` and
   `ItemKind` are appended at the end, and every `match` gets its new arms in the same commit.
7. combat.rs ownership by function within a wave: wave 1 has P3 (wildlife branch → moved out)
   and P5 (siege branch, `check_rout`, `siege_strength`); wave 2 has P9 (`strike`,
   `do_attack`, `combat_awareness`, siege targeting) and the coordinator merges P3 and P5
   before starting P9.
8. `update_dwarf` gets two hook calls in P0 and stays at that: `self.body_phase(d)` after the
   body tick (P7's press and carry, P4's berserk go inside, in their own files) and the
   existing `psyche_tick`.

## Work packages and waves

| Pkg | Items / steps | Sessions | Wave | New files | Shared edits (smallest footprint) |
|---|---|---|---|---|---|
| P0 | S10 OTH-04, all refactors | 1.5 | 0 | schedule.rs, finders.rs, overseer_hooks.rs, knobs.rs | game.rs, overseer.rs, ambitions.rs, dwarf.rs, annals.rs, report.rs, main.rs |
| P1 | S02 step 4 (HLT-03, MIL-04a) | 1.5 | 1 | — | body.rs; 1 line each in game.rs: update_dwarf, combat.rs: creature_tick; medicine.rs: Suture |
| P2 | S07 MIL-01, MIL-02 | 2 | 1 | readiness.rs | buildings.rs Recipe (append), dwarf.rs 1 field, military.rs try_shoot, combat.rs do_equip/equip callers, troubles.rs (append), overseer.rs ov_orders |
| P3 | S08 CRE-01, -03, -15 | 2 | 1 | wildlife.rs | combat.rs creature_think (wildlife tail out), wildlife(), creature_tick 1 call; creature.rs 3 fields |
| P4 | S09 MND-06, MND-31 | 2 | 1 | breakdowns.rs | dwarf.rs 1 field; game.rs update_dwarf (token + 3 lines); combat_awareness 1 call; medicine.rs 1 cond; finders/schedule rows |
| P5 | S03 captives | 1.5 | 1 | captives.rs | traps.rs 1 call; combat.rs check_rout, siege_strength, maybe_siege text, siege branch 1 call; annals.rs SiegeNote field |
| P6 | S05 wells and cisterns (ENG-18) | 1.5 | 1 | cistern.rs | overseer_hooks 2 lines; troubles.rs fix arm |
| P13 | S06 (OUT-17, -19, -20) | 1.5 | 1 | — | expeditions.rs, ambitions.rs ov_venture, ui.rs 1 line |
| P7 | S02 steps 1–2 (HLT-07) | 2.5 | 2 | rescue.rs | finders rows; dwarf.rs 2 fields; update_dwarf via body_phase |
| P8 | S02 step 3, HLT-20, -21, -47 | 2 | 2 | — | overseer.rs plan/ov_labors/ov_orders/ov_fit; items.rs StockFilter (append); troubles.rs Lack::Doctor; medicine.rs Feed; ui.rs Health page; dwarf.rs med |
| P9 | S07 MIL-05, MIL-04b | 2 | 2 | — | combat.rs strike/do_attack/combat_awareness/siege targeting; overseer.rs ov_alert station, ov_army uniform |
| P11 | S07 MIL-03 | 1 | 2 | hunts.rs | overseer_hooks 1 line; Game 1 field; finders guard |
| P12 | S04 learn (SIE-01) + step 0 | 2 | 2 | lessons.rs | overseer.rs Overseer field, ov_alert patience, ov_defences; annals.rs note_death 1 line; hooks |
| P14 | S08 CRE-02, -22, -23, -29 | 2.5 | 3 | — (wildlife.rs) | fire.rs 1 line; Game 1 field; schedule 1 line; hunts.rs lair target |
| P15 | S10 OTH-06, knob wiring, tuning | 2 | 3 | — | creature.rs 1 field; combat.rs backoff; finders wrapper |

**Wave 0:** P0 alone (serial).
**Wave 1 (7 parallel):** P1, P2, P3, P4, P5, P6, P13. Conflict notes:
- P2 and P5 both append to `troubles.rs: Lack`? No: P5 doesn't. P2 and P6 both add `fix`
  arms (different arms), a trivial merge.
- P3 and P5 both edit `combat.rs: creature_think`, in different branches. Merge P3 first.
- P1, P4 and P7 all touch `update_dwarf`. P1 changes one line and P4 changes the sipping
  token plus the tantrum branch, so they merge cleanly. Merge order: P1 → P4.
**Wave 2 (5 parallel):** P7, P8, P9, P11, P12.
- P8, P9 and P12 all edit `overseer.rs`, in disjoint functions (P8: plan/ov_labors/ov_orders/
  ov_fit; P9: ov_alert's station lines and ov_army's uniform line; P12: ov_alert's patience
  line, ov_defences, Overseer struct). P9 and P12 both touch `ov_alert`, so merge P12 first
  and let P9 rebase. Alternatively, P12's patience change goes through a helper
  `fn bar_patience(&self, o) -> u64` that P0 extracts in advance; recommended.
- P7 must merge before P8's step 8 and its UI work, which shows bandaged and pressed state.
**Wave 3 (2 parallel):** P14, P15. P15's tuning step (P15-c) runs last, after P14 is merged.

Coordinator batches: one 25-fort batch after each wave (about 40 minutes each), compared with the
previous wave's file (`batches/m1-w1.json` …). The 50-fort release batch comes after wave 3.

## Verification per package

| Pkg | Tests the agent adds (must pass with `cargo test --release`) | Batch after merge? |
|---|---|---|
| P0 | `schedule::same_story_after_refactor` (golden), `knobs::a_knob_changes_the_game`, `report::counters_are_summed_and_compared`, save-fixture load | 5-fort batch identical to before (coordinator) |
| P1 | `body::one_gash_lying_still_clots`, `a_severed_arm_kills_in_minutes`, `five_gashes_fighting_kill_in_hours`, `old_saves_bleed_on_the_upper_body`; updated `medicine::bleed_out`; arena table in the commit | wave-1 batch |
| P2 | `industry_tests::forge_makes_iron_bolts_and_crossbows`, `military::iron_bolts_hit_harder`, `readiness::kit_is_on_before_the_first_siege` | wave-1 batch |
| P3 | `wildlife::wolves_pass_a_squad_and_take_the_straggler`, `a_wounded_wolf_breaks_off`, `the_pack_scatters_when_one_dies`; existing wildlife tests unchanged | wave-1 batch |
| P4 | `psyche_tests::a_raving_dwarf_still_drinks_when_desperate`, `breakdowns::a_berserk_dwarf_is_subdued`, shift test, catatonia test | wave-1 batch |
| P5 | `captives::a_siege_half_caged_routs`, fate tests (Bold executes, Cautious questions), `besiegers_free_their_own` | wave-1 batch |
| P6 | `cistern::the_overseer_sinks_a_well_over_water`, `a_cistern_fills_and_its_gate_shuts`, `sheltering_dwarves_drink_from_the_well` | wave-1 batch (watch drowning) |
| P13 | `expeditions::no_expedition_from_a_weak_fort`, `a_losing_squad_comes_home_early_with_its_wounded`, rations/tactician test | wave-1 batch |
| P7 | `rescue::friends_bandage_the_wounded_where_they_lie`, `the_unconscious_are_carried_to_hospital`, `nobody_walks_into_a_fight_to_carry`, `three_gashed_dwarves_are_carried_in_and_sutured` | wave-2 batch |
| P8 | `overseer::a_hospital_before_the_first_siege`, `medicine::a_patient_in_bed_is_brought_ale`, `history_records_care`, Health page render test | wave-2 batch |
| P9 | `combat::a_downed_goblin_is_finished`, `goblins_leave_the_fallen_for_the_standing`, `military::a_marksdwarf_backs_off_and_shoots`, `combat::soldiers_in_metal_kill_their_goblins`; arena rerun in the commit | wave-2 batch |
| P11 | `hunts::the_overseer_sends_a_squad_after_wolves`, `reckless_never_hunts` | wave-2 batch |
| P12 | `overseer::the_gate_is_whole_by_the_first_siege`, `lessons::a_siege_that_got_in_brings_traps_and_a_second_squad`, watch test, sense test | wave-2 batch |
| P14 | `wildlife::three_dwarves_together_drive_off_a_wolf`, silent-woods log test, `packs_come_from_and_return_to_their_lair`, `a_hunted_out_lair_sends_nothing` | wave-3 batch |
| P15 | `knobs::siege_size_cap_is_a_knob`, `path_throttle::a_creature_shut_out_searches_less`, golden story within tolerance, `overseer_speed` timing | wave-3 batch + 50-fort release batch |

## Estimated effort

P0 1.5 + P1 1.5 + P2 2 + P3 2 + P4 2 + P5 1.5 + P6 1.5 + P13 1.5 + P7 2.5 + P8 2 + P9 2 +
P11 1 + P12 2 + P14 2.5 + P15 2 = **about 27.5 agent sessions**. With the waves run in
parallel, that's about 8–9 sessions of wall-clock time: wave 0 is 1.5, wave 1 is ≤ 2, wave 2 is
≤ 2.5 and wave 3 is ≤ 2.5, plus four coordinator batches.

## Cross-milestone notes

- **Counters and knobs (P0) are the shared measuring tape.** Every later milestone should report
  through `Annals.counters` and tune through `Game::knob`, never new `Trial` fields or
  hard-coded constants. M2's O-items (personalities, plans) get A/B tests for free.
- **The finder table and overseer hooks** are the main merge-conflict cure for every later wave
  (M3 society systems add finders such as tavern, council and justice; M4–M5 add overseer
  hooks). Do P0 well.
- **A "memory of events" store is forming in three places:** `Overseer.lessons` (P12),
  `Annals.grudges` (P5) and `Dwarf.med` (P8). C02 Grief and vengeance, B15 The caravan
  remembers and B16 Enemies with faces all need "who did what to whom, when". Consider one
  `Memory { tick, who: Actor, what: Deed, whom: Actor, where: Pos }` log in annals with
  per-system views, introduced at M3's start. M1's three stores can migrate to it.
- **An "opinion of civ" store** begins with `Annals.grudges` (captives executed or freed). B02
  Diplomacy should grow it into `civ_regard: BTreeMap<u32, i32>`, not a second map.
- **Creature "mind" fields** (`pack`, `stalk`, `wary_until`, `fears_fire`, `path_fails`) belong
  in a `CreatureMind` sub-struct (`#[serde(default, flatten)]`) when B20–B23 (pests,
  snatchers, beasts, the dead) add more. Do it in P14 if it grows past five fields.
- **`trial_game(seed, style, until)`**, a run_trial that returns the Game at a given day, is
  needed by tests in P8 and P12 and later by O-items. Add it in P0.
- **The arena** (`combat.rs: arena`) should become a non-ignored, fast test with asserted ranges
  (win%, ticks, bled%) once P1 and P9 settle, so balance regressions fail CI rather than wait
  for a batch.
