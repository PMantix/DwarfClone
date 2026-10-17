# Implementation plan: Milestone 3

Planned against the code as of October 2026 (DwarfClone, ~37k lines). File and function references
are to that tree. Nothing in the repo was edited.

## Milestone 3: the fort as a society (v0.5)

### What the code has today (the ground this milestone stands on)

- **Thoughts are untyped strings.** `Dwarf::think(text, stress, tick)` (src/dwarf.rs) scales stress
  by `personality.anxiety`, adds it straight to `d.stress` (floor `STRESS_FLOOR` = -400) and keeps
  the last **8** `Thought { text, stress, tick }`. There are about 100 `.think(` call sites, and
  several systems dedupe by matching the text within a time window (family.rs `haunt`,
  weather.rs, miasma.rs, megaprojects.rs). Tests assert on thought text in about 20 places.
- **Stress** decays 1 point per 100 ticks toward 0 (game.rs `update_dwarf`, 12 a day). A tantrum
  fires at `450 - temper*2` (dwarf.rs `tantrum_threshold`). Melancholy follows sustained misery
  (stress ≥ 650) in `psyche_tick`. There is no short-term mood and no memory.
- **Death grief is immediate and fort-wide.** `Game::kill` (game.rs) gives every living dwarf a
  thought: spouse +140, child +150, parent +110, friend (regard ≥ 30) +90, enemy -10, and
  **+25 to everyone else, whether or not they saw or heard anything**. A siege that kills 8 adds
  about +200 stress to a dwarf who knew none of them, enough to start tantrum cascades.
- **Relationships** are one number: `relations: Vec<(u32, i32)>`, regard from -100 to 100, never
  decays and is never pruned of the dead. `socialize` (psyche.rs) talks to the *first* dwarf within
  3 tiles, so low indices are favoured; compatibility looks only at sociability.
- **Family** (family.rs): spouse, `parents: Option<(mother, father)>`, births, weddings between
  dwarves with regard ≥ 70 (no ceremony), aging, ghosts, burial. No siblings, grandparents or
  adoption. `new_child` takes the mother's family name and nothing else from the parents.
- **Drama** (drama.rs `drama_daily`): pranks, love triangles, insults between mutual grudges
  (≤ -40), a duel at dawn (`Duel`, `fight_duel`); `gossip: Vec<(u64, String)>` is shown in the
  status screen's Gossip tab.
- **Justice** (justice.rs): `Crime { kind: Assault|Vandalism|Disobedience, culprit, victim, what,
  tick, punished }`. The culprit is always known. Crimes come only from tantrum mischief and failed
  mandates. The sheriff beats the culprit or jails them for 3 days a charge (`punish`, `serve_sentence`).
- **Nobles** (psyche.rs `nobles_daily`, `issue_mandate`, `check_mandate`; positions.rs
  `titles_daily`, `make_demand`, `judge_demands`): there is one `mandate` slot; when it goes unmet
  a random dwarf is blamed and charged with Disobedience. **The overseer never reads mandates or
  demands.** The barony is handed out automatically (to the most sociable dwarf).
- **Needs** (needs.rs): seven needs; Temple and Tavern zones meet Pray and Party (`zone_need`,
  `seek_need_zone`, `on_break`). There are no gods. Drinking (game.rs `do_task`
  `Task::DrinkItem`) sets thirst to 0 and meets the Drink need; there is no intoxication.
- **Festivals** (festivals.rs): Founding, Harvest, Midwinter, Dedication, each with dancing, a
  drinking contest (`mugs`) and arm-wrestling.
- **Visitors** (visitors.rs): bards and wandering warriors come to the tavern; strangers come from
  mysteries.rs.
- **Strange moods** (psyche.rs `start_mood` / `do_mood`): a fey mood only. A mood that fails
  sends the dwarf mad.
- **The overseer** reads troubles (`troubles.rs Lack`, `attention`, `fix`), a brain of 13 senses
  (`ambitions.rs Sense`, `brain()`), and has 8 traits (`personality.rs Trait`) that lean its
  ambitions. **None of its senses is social, apart from Unrest (mean stress).**
- **The batch can't see society.** `Trial` (overseer.rs) and report.rs count deaths, sieges and
  survival. Tantrums, crimes, weddings and madness are invisible to `--compare`.

### Order and why

Build order: **C00 foundations → C01 (part 1) → in parallel C03 core, C05, C08, C10 (part 1),
C11, C06 (part 1) → C01 (part 2), C02, C04, C07 (part 1), C09, C06 (part 2), C10 (part 2) →
C07 (part 2), C12.**

- **C00 (new, about 3 sessions)** is not in the WISHLIST. It does the hot-spot refactors and lays
  down the shared types every item writes to: the structured feeling, happenings with
  witnesses, kin, relationship dials, petitions, the society tally in the batch report, and stub
  hooks. Without it, the 12 items each invent their own event and memory scheme and fight over
  `game.rs`, `dwarf.rs`, `psyche.rs` and `ui.rs`.
- **C01 part 1 comes first** (richer thoughts, many emotions, two-speed feelings, stress
  tuning). Every later item emits feelings, and the stress model decides whether M3 makes forts
  more fun or just more suicidal. Its retune of grief also removes the fort-wide +25 per death.
- **The critical path runs C00 → C01a → C03 brawls → C07 justice I → C07 justice II.** Brawls are
  the shared violence engine for feuds, drunks (C08/C09), goading (C04), tantrums (C12) and prison
  riots (C07). Justice needs witnesses (C00), suspicion and trust (C03) and crowds (C04).
- **Can run in parallel after C00 and C01a:** C05 (family), C08 (bottle), C10a (gods), C11 (food)
  and C06a (mandates) each live mostly in their own new module.
- **Dependencies on other milestones:**
  - **M1 S09 "Madness that mends"** (berserk, catatonia) changes `go_insane` and tantrums. C12
    must come after it. C01's two-speed feelings must be tuned after S09, or the batch numbers
    mix two changes.
  - **M1 S10 "Knobs for the batch"**: if it has landed, every M3 threshold (stress, grief, bac,
    shunning) goes in its tunables file. If not, keep the constants together in `memory.rs`.
  - **The same mechanic is listed twice; build each once.** Last words (MND-12 here, UI-02 in O05).
    Spin (NOB-09 here, OVR-05 in O07). Unit list with stress (MND-20 here, UI-13 in O06). Life
    story (MND-02 here, UI-05 in O06). Mandates feed orders (NOB-01 here, ECO-35 in O08). Revenge
    (MND-05 here, MIL-13 in A03). Barony, banishment and petitions (NOB-11, NOB-03, OVR-30 here;
    OVR-09, OVR-08, OVR-11 in B02). Desecration curse (MYT-13 here, N03's "done when").
    Performers (SOC-30 here, F07). Rebellious youth is listed in **both C04 (MND-18) and C05
    (SOC-22)**: build it once, in C05, and tick both. Whichever milestone gets there first owns
    the code; the other reuses it.
  - **M1 and M2 touch the same files** (combat.rs, overseer.rs, ambitions.rs). Start M3 wave 1
    after M1's overseer and combat work has merged, or rebase C00 onto it first.

### Release check

1. `cargo test --release` passes, including every "Done when" test listed under the items below.
2. **Batch** (`--autoplay --runs 25 --years 3 --seed 5000`), compared with the last M2 batch:
   - **survival** at years 1, 2 and 3 no worse than M2, within ±1 fort a year;
   - **social deaths** (murder, alcohol poisoning, a dare or vow gone wrong, execution, the fell
     mood) ≤ 8% of all deaths;
   - **tantrums plus melancholy** per fort-year no more than 25% above M2, and lower in the
     first year (the grief retune);
   - **breadth**: in ≥ 60% of forts that reach year 2, the new Society section shows at least one
     wedding, brawl, crime with a witness, mandate answered, petition answered, temple dedication
     and relived memory; ≥ 30% show a vow, a drunk brawl and a named brew or dish.
3. **Speed:** `one_year_timing` within +10% of M2.
4. **Things to watch:** one autoplay fort's chronicle should tell a social story with no siege in
   it (a wedding, a feud, a theft trial). The dwarf sheet of a massacre survivor should show a
   relived memory and a trait change.

---

### C00 Social foundations  (effort: 3 sessions; risk: med)

Not in the WISHLIST; this is the groundwork. See "Interface contracts" and "Hot-spot discipline"
below for the exact signatures.

- **Design:**
  - **Hot-spot refactors** (pure moves, no change in behaviour): H1–H9 below. psyche.rs is split
    into council.rs, moods.rs and social.rs. `kill`'s grief block moves to `grief.rs::on_death`.
    `idle_activity` becomes `leisure.rs::leisure`. `Dwarf::think`'s body becomes `feel` in
    memory.rs. A new society.rs holds the registry of hooks.
  - **memory.rs:** `Emotion`, `About`, `Feeling`, `Memory`, `InnerLife`, `HapKind`, `Happening`,
    `Game::happen` (records the happening, finds witnesses within 8 tiles on the same level,
    dispatches it to reactor stubs), `Game::witnesses`. `Thought` gains three `#[serde(default)]`
    fields: `emotion`, `about`, `at`.
  - **Per-system sub-structs on Dwarf**, each defined in its own module and `#[serde(default)]`:
    `inner: InnerLife` (memory.rs), `social: Social` (social.rs), `saga: Saga` (family.rs),
    `drink: DrinkState` (bottle.rs), `faith: Faith` (faith.rs). After C00, **no package edits
    dwarf.rs**; they grow their own struct.
  - **Game fields:** `happenings`, `next_hap`, `brawls`, `council`, `pantheon`, `tavern`,
    `kitchen`, `grief`, `exiles`. Each type has `Default` and lives in its own module.
  - **Enum variants added up front**, each with a name and inert behaviour:
    - `Task::Errand(Errand)`;
    - `CrimeKind::{Theft, Murder, Drunkenness, Desecration, Bribery, Escape}`;
    - `ZoneKind::Arena`; `BuildingKind::{Altar, Fishery, Stocks}`;
    - `Position::{Hammerer, TavernKeeper}`; `FestivalKind::{Victory, Wedding}`;
    - `Sense::{Mourning, Lobby, Discord, Devotion}`; `Lack::Mandate`; `MoodKind`, as the new
      `Mood.kind` field.
  - **kin():** derived from `parents` and `spouse`, plus `saga.kin`.
  - **Ties:** `Social.ties` with trust, respect, fear and impression dials. `relations` keeps regard.
  - **Petitions:** `Council.petitions`, `petition()`, `answer_petition()`, `overseer_grants()`.
  - **Society tally:** `Annals.society: BTreeMap<String, u32>` with
    `Game::tally(key: &'static str)`. `Trial.society` copies it. report.rs gets a "Society"
    section (the mean per fort of every key) and `--compare` shows it. Using string keys means
    packages add counters without editing a struct.
  - **UI:** a new src/ui_society.rs with `sheet_extra(g, d) -> Vec<Line>`, `status_society(g) ->
    Vec<Line>` (a sixth status tab, "6 Society") and `council_panel(g) -> Vec<Line>` (the Nobles
    screen). ui.rs and app.rs get one call each, and the tab count becomes a const.
- **Steps:**
  1. Refactor commit: split psyche.rs; `on_death`; `leisure`; `feel`; society.rs stubs wired into
     `step` (3 lines), `ov_daily` (1 line), `brain()` (2 lines), `do_task` (1 arm), `kill` (1 line).
     · psyche.rs, council.rs, moods.rs, social.rs, grief.rs, leisure.rs, society.rs, game.rs ·
     test: the existing suite passes unchanged, and `one_year_timing` is within 2%.
  2. memory.rs types, `happen`, `witnesses`; Thought fields; `Dwarf.inner`. Emit happenings from
     the existing events: death (`on_death`), siege start and end (`annals_tick`), artifact
     (`do_mood`), wedding and birth (`families_daily`), crime (`report_crime`), festival.
     · test `memory::tests::a_death_is_witnessed_only_by_those_nearby`: a dwarf 3 tiles away is a
     witness, one 30 tiles away isn't, and the ring keeps the last 300.
  3. Sub-structs and Game fields, enum pre-adds, kin(), ties, Council and petitions.
     · tests: `family::tests::kin_follows_parents_and_spouse` (sibling, grandparent, child);
     `council::tests::a_petition_is_answered_once`; and a save from before this commit loads (the
     `saves::tests` pattern on a serialized `Game::new` with the new fields stripped).
  4. Society tally, the report section, `--compare`, ui_society scaffolding and the "6 Society"
     tab. · test `report::tests::society_counts_are_reported_and_compared`.
- **Overseer & screen:** the `ov_society(o)` hook and the 4 Sense variants, firing at 0 for now. A
  Society tab that lists the year's tallies.
- **Measure:** a re-baseline batch (behaviour unchanged, so survival matches M2 exactly with the
  same seeds) saved as `batches/m3-base.json`. It is the first batch with society counts.
- **Risks / open questions:** pre-adding enum variants before their logic exists. Each is inert:
  a building with no recipe, a position with no duty yet. **Decision for the user:** keep
  `relations` for regard and add `ties` alongside (recommended, it touches 0 call sites), or
  migrate to a single `Vec<Tie>` (cleaner, 40+ call sites).
- **Parallel work:**
  - **Writes:** almost every hot spot, once. New files: memory.rs, social.rs, council.rs,
    moods.rs, grief.rs, leisure.rs, society.rs, brawls.rs, bottle.rs, faith.rs, tavern.rs,
    kitchen.rs, errands.rs (stubs) and src/ui_society.rs.
  - **Reads / depends on:** M1 S09 and the M1 overseer work merged first, if possible.
  - **Hot spots:** all of them. That is why one agent does it alone in wave 0.
  - **Agent size:** 2 agents, run serially: steps 1–2, then steps 3–4.

---

### C01 The inner life  (effort: 5 sessions; risk: high, balance)

- **Design:**
  - **Richer thoughts (MND-23):** call sites that know who, what or where switch from `think` to
    `feel(Feeling::new(text, s).emo(..).about(..).at(..))`. About 30 call sites get this:
    deaths, combat, drama, family, justice, heroes, pets. The rest keep `think`
    (`Emotion::None`). Reports and the overseer can then quote a thought: "Kelda is grieving for
    Ulmek (killed by a goblin at the gate)".
  - **Many emotions (MND-22):** `memory.rs::emotions_for(kind: HapKind, rel: Option<Kin|tie>,
    p: &Personality) -> Vec<(Emotion, i32)>` gives a small table of reactions. A death seen
    gives grief (scaled by kin or regard), horror (scaled by anxiety) and rage (scaled by temper).
    A broken siege gives pride and relief. The happening reactor `inner_react` turns each
    witness's emotions into one feeling each, so up to 3 thoughts per event.
  - **Two-speed feelings (MND-01):**
    - The new `InnerLife.spirits: i32` (named so to avoid a clash with `Mood` and `Dwarf::mood()`)
      takes each feeling in full. It decays 8% every 100 ticks (in `society_dwarf` through
      `inner_tick`).
    - `stress` takes only `feeling.stress / 4` immediately, plus a slow drip: every 100 ticks,
      `stress += (spirits - 60).max(0) / 25` and, on good days, `-= (-spirits - 60).max(0) / 25`.
    - Big feelings (`|s| ≥ 100`: death of kin, artifact, wedding) still hit stress at 60%.
    - The result: one bad day shows in spirits ("Upset" on the sheet) but only weeks of it break
      someone.
    - `Dwarf::mood()` gains a warning band: **"Close to breaking"** at 80% of
      `tantrum_threshold`, and "Withdrawn" when misery > 0.
  - **Fort mood at a glance (MND-20):**
    - a row of 7 faces (count per mood band) on the status line, through
      `ui_society::mood_strip(g)`;
    - the Dwarves tab (status tab 4) sorted by stress, worst first, with a stress number;
    - a "Thoughts" section on the dwarf sheet with emotion words and causes ("grief, at Ulmek's
      death").
  - **Memories that come back (MND-02):**
    - `inner.remember(&Feeling, tick)` keeps a feeling with `|stress| ≥ 40` in `short` (8 slots,
      the strongest win).
    - Daily `inner_daily`: each short memory older than a season has a 1-in-60 chance a day of
      being relived. That gives a feeling at 30% of the original strength, with the same emotion
      ("relived the death of Ulmek"), and `relived += 1`.
    - At `new_year`, a short memory older than a year **fades** (chance falls as `|stress|` and
      `relived` rise) or **becomes core** (8 lifelong slots). Core memories are relived at 1 in 120.
    - `inner.life` gets one line per season from the strongest new memory ("Spring 1053: saw
      Ulmek fall at the gate"). This is the "life story" O06 UI-05 also wants: one list, two
      screens.
  - **Hardened by tragedy (MND-04):** each death a dwarf witnessed adds `deaths_seen` (2 for a
    friend). `hardened = min(80, deaths_seen*4)`. Grief and horror feelings are scaled by
    `(100 - hardened)/100`, with a floor of 40%. Above 50 hardened, joy from feasts and chats is
    also scaled down by 15% ("grown hard"). Working outdoors for more than a year (counted by
    `inner_tick` when `world.outside`) cuts weather-thought stress by half.
  - **Stress tuning (MND-59):**
    - feelings with `|s| < 5` touch spirits only;
    - migrants arrive with stress 0 and spirits +20 (in `migrants`, through the new
      `on_arrival` hook);
    - a feeling of type Fight/Rage when defending a friend in combat, or `Task::Attack` while a
      friend is within 5, is halved;
    - a dwarf above 500 stress takes double benefit from comfort feelings (C04 altruism, C10
      sermons, C02 grave visits), "the fragile can still be helped";
    - the grief retune: strangers get nothing unless they witnessed the death (MND-21 delivers the
      rest).
  - **Personality drift (MND-03):** when a memory fades or becomes core, `drift_rule(emotion,
    relived, stress)` may shift one facet by ±3..±6: shame that was laughed off lowers anxiety,
    horror relived 3+ times raises anxiety, grief made core lowers sociability, pride raises
    diligence, rage raises temper, love raises sociability. It is clamped to ±50 and logged in
    `inner.changes` ("Since the siege of 1053 (relived 3 times), more easily stressed"), which the
    sheet shows under "How life has changed them".
- **Steps:**
  1. Feeling call sites and the `emotions_for` table (MND-23, MND-22). · memory.rs, game.rs
     combat sites through grief.rs, drama.rs, heroes.rs · test
     `memory::tests::a_death_seen_stirs_grief_horror_and_rage_by_personality`: an anxious witness
     gets more horror, a hot-tempered one more rage.
  2. Spirits and the slow drip (MND-01). · memory.rs `inner_tick`, dwarf.rs `mood()` band
     (one line, through a helper in memory.rs) · test `memory::tests::a_bad_day_is_not_a_ruin`:
     five +20 feelings in a day leave stress < 60 but spirits "Upset"; the same every day for 3
     weeks reaches "Close to breaking".
  3. Stress tuning (MND-59). · memory.rs, society.rs `on_arrival` · test
     `memory::tests::trifles_dont_stick_and_migrants_arrive_calm`.
  4. Fort mood at a glance (MND-20). · ui_society.rs, ui.rs status tab sort (1 call) · test
     `ui_society::tests::mood_strip_counts_every_band` (pure function over a `Game`).
  5. Memories that come back (MND-02). · memory.rs `remember`, `inner_daily`, the `new_year` hook
     · test `memory::tests::a_massacre_is_relived_a_season_later`: put 3 deaths before a
     witness, run 2 seasons with a fixed rng, check for a "relived" thought and a memory in `short`.
  6. Hardened by tragedy (MND-04). · test `memory::tests::the_tenth_death_hurts_less_than_the_first`.
  7. Personality drift (MND-03) and the sheet section. · memory.rs, ui_society.rs · **"Done
     when" test** `memory::tests::a_survivor_comes_out_changed`: a dwarf who witnessed a massacre
     relives it, and after the year turns their anxiety has moved and `inner.changes` names the
     siege.
- **Overseer & screen:**
  - The brain's `Unrest` sense reads mean *spirits* as well as stress, so the overseer reacts
    sooner and with less.
  - A new trouble when ≥ 3 dwarves are "Close to breaking". `ov_society` → `ov_comfort`: a Cheerful
    or Sentimental overseer starts a small feast (a `Festival` with no calendar day), a Pious one
    raises temple priority, and the rest note it.
  - On screen: the mood strip, the sorted Dwarves tab, the Thoughts, Memories and "How life has
    changed them" sections, and a spotlight when a core memory forms ("Kelda will never forget
    the fall of the gate").
- **Measure:** tantrums per fort-year (`tally("tantrum")`) and melancholy; target −20% in year 1
  compared with m3-base, and survival no worse. `tally("relived")` ≥ 5 per fort-year.
- **Risks / open questions:** two-speed feelings change every balance number. Do them right after
  S09, and batch both orderings if unsure. Thoughts stay capped at 8 so the text-dedupe windows
  keep working; memories are separate. Drift must stay slow (at most one change per dwarf per
  year), or personalities random-walk.
- **Parallel work:**
  - **Writes:** memory.rs (`inner_*`, `emotions_for`, `drift_rule`), ui_society.rs (mood strip,
    sheet sections), and `feel` swaps at call sites in drama.rs, heroes.rs, pets.rs, grief.rs,
    combat.rs (3 lines).
  - **Reads / depends on:** C00 (`Feeling`, `Happening`, the `inner_react` stub, the `on_arrival`
    hook).
  - **Hot spots:** dwarf.rs `mood()`, a one-line delegation; combat.rs, 3 call-site swaps only.
  - **Agent size:** 2 packages: P1 (steps 1–4) and P8 (steps 5–7).

---

### C02 Grief and vengeance  (effort: 4 sessions; risk: med)

- **Design:**
  - **Grief needs a body (MND-21), together with C07's NOB-06 "Missing, then dead":**
    - `grief.rs::on_death` no longer grieves anyone who wasn't there. If the death had witnesses,
      kin and friends among them grieve at once and the rest of the kin are told "heard that X
      died" (full grief, a day later through `GriefBook.pending`).
    - If nobody saw it (no witness within 8: a lone woodcutter, a fall), `GriefBook.missing` gets
      `{dead, since, corpse_item}`. Kin get "worried that X has not come back" (+10). Every 50
      ticks, `grief_tick` checks the few missing corpses against dwarves within 3 tiles; the
      finder gets "found the body of X" (horror, scaled for strangers: +8), and grief is released
      to kin.
    - After 10 days unfound, the dwarf is declared dead ("is presumed dead") and grief is released
      at 70%. Bones from `rot_daily` (labelled "remains of X") count as a body ("even a tooth").
    - The status Health tab's dead list shows "missing" until then.
  - **Grief wandering (MND-11):** for one season after a death, spouses, children, parents and
    close friends (regard ≥ 70) take one leisure trip a day (`leisure` chain → `grief_visit`)
    through `Task::Errand(Errand::Mourn { dead })`. They go to the coffin or slab holding the dead
    (`Building.occupant`), else the place they died, else the workshop of the dead's best labor.
    They linger there 60–200 ticks and get "sat by X's grave" (-6, a comfort). The visit is the
    cure: each one takes 15% off the grief memory's strength.
  - **Last words (MND-12, shared with O05 UI-02):** `on_death` picks a line from templates by
    circumstance and personality. A spouse alive: "Tell Dagna I…". A killer known and temper > 20:
    a curse at the goblins. A hero: defiance. Anxious: fear. Old age: peace. Alone: no words (no
    witness). The line goes to the announcement, `DeathNote.last_words` (`#[serde(default)]`), the
    legend line from `hero_fell` or a new `legend` for any named death, and `year_log` for the
    chronicle.
  - **Vengeance vow (MND-05):**
    - On a death with a killer (`annals.wounded_by` kind, plus `d.attacker` creature id), each
      spouse, parent, child or close friend with `temper > -10` swears a vow with chance
      `0.3 + temper/100 + 0.2*(kin)`: `social.vow = Some(Vow { foe: kind, foe_id, for_whom, sworn })`.
    - **Effects:** (a) in `combat_awareness`, an armed vowed civilian adjacent to a creature of
      that kind attacks instead of fleeing; (b) a vowed soldier's `Task::Attack` target prefers
      `foe_id`, then the kind (`vow_target` hook); (c) `ov_army` drafts vowed adults first
      (a score bonus in the draft ranking).
    - **Kept:** `credit_kill` of that kind or id gives "avenged X" (pride −60, relief), a legend, a
      gossip line, epithet "Avenger" (`RANK_FEAT+1`), and ends the grief memory's reliving.
    - **Broken:** after 2 years, shame (+30, a core memory, which feeds MND-03 drift).
  - **Traumatic triggers (MND-07):**
    - `GriefBook.dark_places: Vec<(Pos, dead, tick)>` keeps the 30 most recent deaths. A close
      friend of the dead who passes within 2 tiles gets "was reminded of X's death" (once a
      day); the leisure chain skips meeting tiles within 3 of a dark place for them for a season.
    - A memory with Emotion::Horror about a foe kind or fire (fire.rs burns, dragon breath) makes
      a dwarf panic when the same kind comes back: the flee radius goes from 7 to 12, they get
      "panicked, remembering the dragon" (+15), and they won't take `Task::Attack` from a vow
      against it. This is a `trauma_flee_radius(d)` hook in `combat_awareness`.
  - **Buried resentment (MND-17):**
    - Slights become core memories with `About::Dwarf(slighter)`: blamed for a mandate
      (`check_mandate`), beaten or jailed (`punish`), passed over for a post (`appoint` when a
      candidate ranked above the appointee exists), lost a duel, the butt of an unamused prank.
    - `resents(d, other) -> bool` (memory.rs) is true while such a memory is core and regard < 20.
    - It surfaces at the worst moment: in a brawl involving `other`, the resentful joins against
      them (C03 reads it); as a witness against `other`, they lie (C07 reads it); in a siege, a
      resentful doctor won't treat `other` (medicine.rs: one check in the doctor's patient
      choice); in an election or petition, they speak against `other` (C06).
    - Each surfacing gets a gossip line naming the old slight: "Sorli has not forgotten the day
      Kelda had them beaten".
  - **Coming home (MND-37):** expeditions.rs `expedition_returns` calls `homecoming(&mut d,
    away_since)`. It delivers held grief (`GriefBook.pending` keyed to members who were away),
    news of births and weddings among kin (joy), and a reunion with a spouse or children (−25).
    Pride or shame already exist. A spotlight is shown if someone comes home to a death.
- **Steps:**
  1. MND-21 / NOB-06 missing and found. · grief.rs, miasma.rs (bones count, 1 line) · test
     `grief::tests::grief_waits_for_the_body`: kill a dwarf with nobody near; kin aren't
     grieving; a dwarf walks onto the corpse; kin grieve.
  2. Last words. · grief.rs, annals.rs `DeathNote.last_words` · test
     `grief::tests::a_witnessed_death_leaves_last_words_in_the_legends`.
  3. Grief wandering. · grief.rs, errands.rs `Errand::Mourn`, leisure.rs (1 line) · test
     `grief::tests::the_widow_visits_the_coffin`.
  4. Vengeance vow, the vow parts. · grief.rs, social.rs `Vow`, combat.rs (2 hooks),
     overseer.rs `ov_army` (1 line, a score bonus) · **"Done when" test**
     `grief::tests::a_widow_swears_vengeance_and_seeks_the_killer`: a spouse killed by a goblin;
     the widow vows; the next siege spawns goblins (`force_siege`); she is drafted and her attack
     target is a goblin.
  5. Traumatic triggers. · grief.rs, combat.rs hook · test
     `grief::tests::a_dragon_survivor_panics_at_the_next_dragon`.
  6. Buried resentment. · memory.rs `resents`, the slight sites (justice.rs, council.rs,
     positions.rs `appoint`, drama.rs) · test
     `memory::tests::an_old_slight_turns_a_witness_against_the_sheriff` (pure: resents() and a
     brawl side choice).
  7. Coming home. · expeditions.rs (1 call), grief.rs · test
     `grief::tests::the_expedition_comes_home_to_a_funeral`.
- **Overseer & screen:**
  - The new Sense::**Mourning** (fresh deaths of named or kin dwarves this season) leans the brain
    toward Revel, Provision and Fortify, and away from Venture. A Sentimental overseer weights it
    ×1.5 and schedules burial and slab work first (ov_burials already exists; add slabs for
    missing dwarves).
  - The overseer drafts the vowed (above).
  - On screen: the dead list ("missing", "presumed dead"), last words in announcements and the
    chronicle, a "Vow: to slay the goblins who killed Ulmek" line on the sheet, and gossip for
    every vow, grave visit and resentment.
- **Measure:** `tally("vow_sworn")`, `("vow_kept")` (target: ≥ 1 vow kept in 30% of forts with 2+
  sieges), `("found_body")` and `("presumed_dead")`. Deaths with cause "vow" (a civilian fought
  instead of fleeing) stay ≤ 2% of deaths.
- **Risks / open questions:**
  - Vowed civilians fighting could undo M1's survival gains. Gate it on a weapon in hand, and
    tune with the batch.
  - Delaying grief for unwitnessed deaths lowers stress overall, which is good, but might hide
    deaths from the overseer. The Mourning sense reads `annals.deaths`, not thoughts.
- **Parallel work:**
  - **Writes:** grief.rs (all), errands.rs (`Errand::Mourn`), combat.rs `combat_awareness` (2
    hook calls), expeditions.rs `expedition_returns` (1 call), overseer.rs `ov_army` (1 line),
    medicine.rs (1 check), annals.rs `DeathNote` (1 field).
  - **Reads / depends on:** C00 (`on_death`, `happen`, `kin`, `GriefBook`) and C01a (`feel`,
    emotions).
  - **Hot spots:** combat.rs, overseer.rs, 3 lines total.
  - **Agent size:** one package, P9 (2 sessions for steps 1–4, then 2 for steps 5–7; one agent can
    do both serially).

---

### C03 Feuds and factions  (effort: 5 sessions; risk: med)

- **Design:**
  - **The brawl engine (brawls.rs, used by C04, C07, C08, C09 and C12):**
    - `start_brawl(a, b, cause) -> Option<id>`. Each brawler gets
      `Task::Errand(Errand::Brawl { brawl })`: step to an adjacent foe and punch every 30 ticks
      (non-edged wound 1–3; a down dwarf is unconscious, not dead).
    - It ends when one side is all down or yields (stress over 300 or hp low), after 400 ticks, or
      when the sheriff arrives.
    - Every blow is an Assault crime (C07 decides whether it gets reported). A death in a brawl is
      Murder.
    - `brawls_tick` runs every 10 ticks over the open brawls only (cheap).
  - **Grudges from pets (SOC-07):**
    - `pets_daily`: a dog may bite a dwarf near it who has a grudge with its owner (or 2% at
      random): a small wound, "was bitten by Biscuit", regard toward the owner −15.
    - `pet_died` (pets.rs): if the killer creature is someone's pet (when B20 "pets eat pets"
      lands) or a dwarf in a tantrum, the owners feud (−40 both ways).
    - Gossip for both.
  - **Slow road to violence (SOC-08):** `Tie.feud` (0..5) replaces drama.rs's straight jump from
    insult to duel. Each daily step needs both regards ≤ −40 and a roll:
    - 1 cold shoulder;
    - 2 shouting (gossip; both +6);
    - 3 slander: tells a third dwarf a lie about the rival (the listener's regard toward the rival
      −10, trust toward the liar recorded);
    - 4 found out: if the listener has trust toward the rival, the liar's lie is exposed (respect −,
      nickname candidate "Forked-Tongue");
    - 5 blows: a brawl (cause Feud), or a duel if one is hot-tempered and the existing duel
      conditions hold.
    - Each step writes a gossip line so the Gossip tab reads like a story.
  - **Shunning and ostracism (SOC-03, MND-16; one mechanism):**
    - `shun_score = 2*crimes in the last year + dwarves with regard ≤ −40 toward them +
      3*(convicted of theft or murder)`. At ≥ 6, `social.shunned_until = tick + SEASON`.
    - While shunned, `socialize` skips them (the other dwarf refuses: "was shunned by X", +6), the
      leisure chain sends them to the quietest walkable tile away from meeting zones, and they get
      +3 stress a day.
    - It ends early with a good deed: a kill in a siege, a festival win, rescuing someone (C04
      altruism). The feedback loop (shunned → stressed → tantrum → crime) is the point, but it is
      capped: a dwarf can be shunned at most once a year.
  - **Factions in a brawl (SOC-04):**
    - Each `brawls_tick`, onlookers within 6 tiles who are awake, calm and not soldiers on alert
      choose. They join the side whose member they regard more if `max(kin bonus 50 + regard) ≥
      40` and temper > −10; resentment (`resents`) counts as +40 against.
    - Or they make peace: temper < −20 and sociability > 20 gives a 25% chance a turn to end the
      brawl, "Hilda pulled them apart".
    - Or they watch: spectators, a crowd for C04.
    - When it ends, every pair across the sides gets −15, and the sides are stored as a `Faction`
      record in `Game.feuds` (feuds.rs) for a year so the gossip and the overseer can name them
      ("the Ironfist kin and the forge crowd").
  - **Nicknames for anyone (SOC-10):** `social.nickname`, earned by counters:
    - survived drowning twice: "Twice-Drowned" (game.rs: drown > 300 then recovered, 1 hook);
    - lost 3 duels: "Yielder";
    - jailed twice: "Jailbird";
    - passed out 3 times: "Barrel" (C08);
    - toppled 2 tables in tantrums: "Tablebreaker";
    - exposed liar: "Forked-Tongue";
    - fell down stairs: "Tumbler" (C08 tripping).
    `Dwarf::called()` shows `Kelda "Jailbird" Ironfist` when there is no epithet (an epithet
    outranks a nickname); this is a dwarf.rs one-line change done in C00. The nickname also goes
    in a gossip line.
  - **Outsider suspicion (SOC-15):** `suspicion(d) -> i32` (social.rs), the hook C07 reads. +20
    if `social.arrived` < 1 year ago, +10 if not dwarf-born or a visitor, +15 for odd habits
    (sociability < −25, or `likes_mat` held by nobody else), plus the shun score. Newcomers start
    with trust −10 from everyone.
  - **Relationship dials (SOC-38):** `Tie { trust, respect, fear, impression }`:
    - **trust** goes up with good chats and honest witness and down with lies, theft and betrayal;
    - **respect** goes up with epithets, masterworks, duels won and bravery, and down with nicknames;
    - **fear** goes up when beaten by them, they hold the hammerer post (C07), or they are big and
      hot-tempered.
    - **Uses:** gossip spreads along trust (`gossip_to(d) -> Vec<u32>`); a witness covers for a
      culprit they trust ≥ 40 or fear ≥ 40 (C07); bribes work on low-trust sheriffs (C07);
      factions join by respect.
    - The sheet shows the strongest tie of each kind ("Trusts Hilda; fears the sheriff").
- **Steps:**
  1. The brawl engine with crowd-free resolution. · brawls.rs, errands.rs `Errand::Brawl` · test
     `brawls::tests::a_brawl_ends_with_someone_down_and_assault_on_record`.
  2. Factions joining, peacemakers, lasting camp grudges. · **"Done when" test**
     `brawls::tests::friends_join_a_brawl_on_their_side`: A and B brawl; C (a friend of A),
     D (kin of B) and E (neutral) are adjacent; C joins A, D joins B, E watches; afterwards C and
     D have a grudge.
  3. Relationship dials and `gossip_to`. · social.rs · test
     `social::tests::trust_rises_with_chat_and_falls_with_lies`.
  4. The slow road to violence. · drama.rs `drama_daily` (replace the insult block), feuds.rs ·
     test `feuds::tests::a_feud_climbs_stage_by_stage_into_gossip`.
  5. Shunning and ostracism. · social.rs, social.rs `socialize` (moved there in C00), leisure.rs
     (1 line) · test `social::tests::a_thief_is_shunned_and_drifts_away`.
  6. Nicknames. · social.rs, hooks in drama.rs and justice.rs · test
     `social::tests::twice_jailed_is_a_jailbird`.
  7. Outsider suspicion. · social.rs · test `social::tests::newcomers_are_suspected_first`.
  8. Grudges from pets. · pets.rs · test `pets::tests::a_dog_bite_starts_a_feud_between_owners`.
- **Overseer & screen:**
  - The new Sense::**Discord** (open feuds at stage ≥ 3, factions, shunned count, brawls this
    season) leans toward Revel (feasts heal) and Fortify (a sheriff and a jail).
  - `ov_society` → `ov_feuds`: with Discord > 0.5, a Cheerful overseer holds a feast, a Cautious
    one makes sure there is a sheriff and a jail (adds `RoomUse::Jail` to `wants`), and a
    Sentimental one moves feuding dwarves' beds apart (reassigning rooms is cheap in
    `assign_rooms`).
  - On screen: the Gossip tab shows the feud ladder, brawl announcements name the sides, the sheet
    shows the nickname and the ties, and the Society tab shows factions.
- **Measure:** `tally("brawl")`, `("brawl_joined")`, `("shunned")`, `("nickname")`. Brawl deaths
  stay ≤ 1% of deaths. Target: 1–4 brawls per fort-year, not 20.
- **Risks / open questions:**
  - Brawls must never pull soldiers off the walls during an alert (skip `d.soldier && alert`).
  - Shunning loops could spiral into melancholy. Watch melancholy in the batch.
- **Parallel work:**
  - **Writes:** brawls.rs, feuds.rs (new), social.rs (dials, shunning, nicknames, suspicion),
    drama.rs `drama_daily` (the insult and duel block), pets.rs `pets_daily` and `pet_died`, and
    game.rs drowning recovery (1 line).
  - **Reads / depends on:** C00 (`Tie`, `Errand`, `kin`, `happen`) and C01a (`feel`).
    `resents()` from C02 step 6 is optional, behind a stub returning false until C02 lands.
  - **Hot spots:** game.rs, 1 line.
  - **Agent size:** P2 = steps 1–5 (3 sessions); steps 6–8 go with P10 or a small follow-up
    package, P2b.

---

### C04 Fort society in motion  (effort: 4 sessions; risk: low)

- **Design:**
  - **Victory cheer (MND-09):** C00 makes `annals_tick` emit `HapKind::SiegeBroken` when the
    outcome becomes "routed". `crowd_react` gives everyone pride and relief (−30), and the squad
    and anyone who killed a besieger −20 more.
    - The overseer (`ov_feasts`): Cheerful, Sentimental or Vain start
      `FestivalKind::Victory` the next morning (festivals.rs: the pre-added variant gets a name and
      text). Greedy gives "extra rations": a thought plus an order for more meals. Stubborn does
      nothing and writes a note.
  - **Goading and dares (MND-13):** in `drama_daily`, a bold dwarf (temper > 20, sociability >
    10) dares a rival or friend.
    - **Accept** if `temper + (respect toward the goader) − anxiety + roll > 30`. The dare is one
      of:
      - fight the goader's rival (a duel);
      - go out and fight a wild animal (`Task::Attack` on the nearest non-legendary hostile
        wildlife within 40 tiles; losing is fun);
      - stand outside the gate during a siege for 200 ticks (`Errand::Dare`);
      - drink 4 mugs at the next festival (C08).
    - **Refuse:** shame (+10) and respect −10 from the goader.
    - Gossip on both. `tally("dare")`, `("dare_death")`.
  - **Envy of rooms, gear and titles (MND-19):** weekly, `society_daily` compares each dwarf with
    friends and rivals. Their bedroom `room_value` less than a third of the other's ("envied X's
    bedroom", +6, regard −5). A soldier with copper or wood beside one with iron. A post given to
    someone ranked lower. Nobles compare only on their rank demand (existing) and offices (C06
    NOB-40).
  - **Impressions and compatibility (MND-26):**
    - `compat(a, b) -> i32` (social.rs): `−sum|facet diff|/10 + 8 for a shared likes_mat + 5 for a
      shared likes_food + 6 for the same best labor + 6 for a shared god (C10)`.
    - `socialize` uses it in place of its sociability-only term. Each chat moves `Tie.impression`
      by compat ± noise; regard moves only by `impression/4` (first impressions stick, but a bad
      week doesn't end a friendship).
    - Partner choice: a random dwarf in range, not the first by index, which fixes the index bias.
  - **Long-lived ties and social clumping (MND-27):** regard doesn't decay (as now). The leisure
    chain's `seek_company`: a sociable idle dwarf (sociability > 10) goes to a meeting tile next to
    their best friend if the friend is in a meeting area. Chat types weighted by personality:
    - jokes (the sociable),
    - shop talk (same labor: Learn need +60),
    - complaining (stress > 150: both +2, trust +3),
    - stories (C09 tall tales),
    - arguing (below).
  - **Arguments (MND-28):** quarrelsome = temper > 20 and sociability > 0. Two quarrelsome dwarves
    argue and enjoy it (−3 each, "had a good argument"); a quarrelsome and a calm dwarf annoy each
    other (+6 to the calm one). This replaces the stress-gated `argue` in `socialize`.
  - **Crowd moods (MND-52):** `crowd(at: Pos, r) -> Vec<u32>` and
    `crowd_feel(members, emotion, s)`. During any gathering, each member's spirits move 20% toward
    the crowd's mean. Gatherings are festivals (every 100 ticks in `festival_tick`), brawls with
    spectators, C07 punishments and C10 sermons.
    - **Cheer:** a crowd mean below −40 gives everyone "cheered with the crowd".
    - **Jeer:** at a punishment, according to the crowd's temper.
    - **Panic:** a horror feeling in a crowd of 5+ spreads at 50%, and those with anxiety > 20
      flee (`Task::Flee` for 100 ticks).
  - **Rebellious youth (MND-18):** built in C05 (SOC-22) and ticked here too.
  - **Altruism (MND-53):** kind = sociability > 20 and temper < 0. In the leisure chain
    (`help_someone`), within 10 tiles:
    - comfort a crying child (a child with stress > 100): both −10;
    - comfort the grieving (a fresh grief memory): −8 and regard +5, counting as a C02 visit;
    - help a passed-out drunk (C08) to their bed with `Errand::Escort`: they walk together and the
      drunk wakes there;
    - sit with the wounded in the hospital: −5 each.
- **Steps:**
  1. Victory cheer and the Victory feast. · festivals.rs (variant text, `start_festival` is reused),
     society.rs `crowd_react`, overseer `ov_feasts` · test
     `festivals::tests::breaking_a_siege_lifts_the_fort_and_a_cheerful_overseer_feasts`.
  2. Impressions, compatibility and partner choice. · social.rs `socialize`, `compat` · test
     `social::tests::alike_dwarves_become_friends_faster`.
  3. Arguments and chat types. · social.rs · test `social::tests::two_arguers_enjoy_it`.
  4. Clumping. · leisure.rs `seek_company` · test `leisure::tests::friends_sit_together`.
  5. Envy. · society.rs `envy_weekly` · test `social::tests::a_poor_room_breeds_envy`.
  6. Goading and dares. · drama.rs, errands.rs `Errand::Dare` · test
     `drama::tests::a_dare_is_taken_or_refused_by_temper`.
  7. Crowd moods. · social.rs `crowd`, the festival and brawl hooks · test
     `social::tests::panic_spreads_through_a_crowd`.
  8. Altruism. · leisure.rs `help_someone`, errands.rs `Errand::Escort` · test
     `leisure::tests::a_kind_dwarf_comforts_a_crying_child`.
  9. **"Done when" test** `social::tests::gossip_shows_envy_and_friendship_without_tragedy`: 60
     peaceful days (`quiet` game) produce gossip with at least one friendship formed and one envy
     line, and no death or siege.
- **Overseer & screen:** Victory feasts (above). `Unrest` now reads spirits, so arguments and envy
  register. For envy, a Vain or Sentimental overseer moves the bedroom plan up its `wants` when
  envy is high (`tally` this season ≥ 3). On screen: gossip ("Kelda and Hilda have become
  friends", "Sorli envies Dagna's bedroom"), cheers and panics in announcements, and the dare in a
  spotlight when accepted.
- **Measure:** friendships formed per fort-year (`tally("friendship")`, regard crossing 50) and
  envy lines. Dare deaths ≤ 1% of deaths.
- **Risks / open questions:** goading civilians into wildlife fights kills dwarves. Cap it at one
  dare a season per fort; the overseer forbids "fight a beast" dares if Cautious. Partner choice
  now costs a random pick among nearby dwarves; keep the 3-tile window.
- **Parallel work:**
  - **Writes:** social.rs (`socialize`, `compat`, `crowd`), leisure.rs (`seek_company`,
    `help_someone`), drama.rs (dares, a separate fn `dares_daily`), festivals.rs (Victory text,
    the crowd hook in `festival_tick`), society.rs (`envy_weekly`, `crowd_react`).
  - **Reads / depends on:** C01a (spirits) and C03 step 1 (brawls, for spectators).
  - **Hot spots:** none beyond the society.rs stubs.
  - **Agent size:** one package, P10 (3 sessions). If C03's P2b is folded in, it is 4.

---

### C05 Family sagas  (effort: 5 sessions; risk: low)

- **Design** (everything in family.rs and the new saga.rs; `Saga { kin: Vec<(u32, Kin)>,
  named_for: Option<String>, guardian: Option<u32>, rite: Option<bool>, favourite: Option<u32> }`):
  - **Named after heroes (SOC-01):** `new_child`'s caller (`families_daily`) has a 35% chance,
    when a hero died in the last 2 years or a living hero is a close friend of a parent, of taking
    the hero's first name. `saga.named_for = Some(hero.called())`, and the legend reads "X was
    born in F to M, and named for Y the Z, who slew …". The chronicle picks it up through
    `overworld.events`.
  - **Weddings as events (SOC-02):**
    - A marriage in `families_daily` schedules `FestivalKind::Wedding` for the next day
      (`Festival.couple: Option<(u32,u32)>`, `#[serde(default)]`; venue as now; length DAY/4;
      guests are the couple's friends and kin, regard ≥ 20).
    - Guests with regard ≥ 30 give a gift (an existing free craft item is moved to the couple's
      room; "gave a gift"). The couple get −30, the guests −10.
    - A jealous rival (regard ≥ 60 toward either, unwed, temper > 0) turns up and insults (C03
      feud stage 2) or starts a brawl at 30%.
    - The legend and chronicle line: "married in F before N guests".
  - **Orphans adopted (SOC-13):** when a child's living parents number 0, `saga_daily` picks the
    kind adult (temper < 20) with the highest regard toward a dead parent (or any kin), who
    becomes `saga.guardian`, `Kin::Ward`/`Guardian`. Both get thoughts, and a legend line.
  - **Siblings, grandparents, heirs (SOC-24):**
    - `kin()` from C00 already derives siblings and grandparents.
    - `migrants()` (game.rs, 1 call to `on_arrival`) gives a 25% chance a newcomer is a sibling or
      cousin of an existing citizen not born here: same family name, `saga.kin` both ways, and
      regard +40 ("was reunited with their sister").
    - Kin react to harm: the `inner_react` hook for Wounded happenings gives kin within sight
      +15 fear or rage.
  - **Mothers and babies (SOC-25):**
    - A baby (age 0) is carried: in `update_dwarf`'s early path (a `child_tick` hook, 1 line), its
      pos copies the mother's; it doesn't path or work, and hunger and thirst are set to 0 while
      with her (nursing).
    - Children of 1–3 wander within 6 tiles of the mother.
    - A mother whose baby is hurt gets horror; one separated from her baby (it was set down at her
      death or a flood) goes to find it (`Errand::Fetch`).
  - **Sibling rivalry and favourites (SOC-23):** yearly, each parent with ≥ 2 children favours the
    one with the highest regard (`saga.favourite`). The others get "felt that X is the favourite"
    (+5) and regard −5 toward the favourite. Siblings defend each other in brawls (kin bonus in
    C03).
  - **Inheritance (SOC-51):**
    - `on_death` → `inherit()`. The owned bedroom passes to the spouse, else the eldest child,
      if theirs is worse (re-pointing `Building.owner`, and the bedroom zone owner).
    - The held weapon or tool and any artifact the dead made are labelled "heirloom of X" (Item
      `label`) and claimed for the heir (`Errand::Fetch` to pick it up).
    - Passed-over children: regard −20 toward the heir, and a slight memory (MND-17).
  - **Heredity (SOC-52):** `new_child` (dwarf.rs, a 6-line change made in C00 through a
    `family::inherit_traits(child, mother, father)` call) makes each facet `(m + f)/2 ±
    rng(−15..15)`, takes `likes_mat` and `likes_food` from a random parent, and the colour from
    the mother 60% of the time. Families come to look alike on the map.
  - **Coming-of-age rite (SOC-12):** at age `ADULT_AGE` in `new_year`, a trial chosen by
    aptitude:
    - "first stone" (carve a craft: needs a Craftsdwarf shop, else rolled);
    - "the long night" (a night alone in the caverns, if any are open; danger);
    - "first hunt".
    Success depends on facets plus a roll: they earn a coming-of-age name (sets
    `nickname`-style "Firststone", not an epithet) and pride. Failure brings shame (a core
    memory). A Happening plus a chronicle line.
  - **Rebellious youth (SOC-22 and MND-18):** children aged 7–11 with temper > 15, once a season:
    - sneak into the barracks to spar (`Errand::Sneak` to a Barracks tile; fight_xp +30; the
      parents get +8 and regard −5);
    - befriend a parent's enemy (regard +20; the parent is furious);
    - wander off outside the walls for 300 ticks (danger by design).
    Gossip for each.
- **Steps** (each one commit, test in `family::tests` or `saga::tests`):
  1. Heredity: `children_take_after_their_parents`.
  2. Named after heroes, **"Done when"** `a_child_named_for_a_fallen_hero_is_in_the_chronicle`:
     kill a dwarf with an epithet; force a birth; check the child's first name, `named_for`, and
     the chronicle line containing "named for" after `write_chronicle_for_test`.
  3. Weddings as events: `a_wedding_is_a_feast_with_gifts_and_maybe_a_rival`.
  4. Siblings, grandparents, migrant kin: `migrant_siblings_know_each_other`.
  5. Mothers and babies: `a_baby_goes_where_its_mother_goes`.
  6. Orphans adopted: `an_orphan_is_taken_in_by_a_friend_of_the_dead`.
  7. Sibling rivalry: `the_unfavoured_resent_the_favourite`.
  8. Inheritance: `the_widow_inherits_the_better_room_and_the_eldest_the_axe`.
  9. Coming-of-age rite: `at_twelve_a_child_faces_a_trial`.
  10. Rebellious youth: `a_hotheaded_child_sneaks_into_the_barracks`.
- **Overseer & screen:**
  - `ov_society` → `ov_saga`. A Sentimental or Cheerful overseer makes room for wedding feasts
    (they come anyway, but it orders extra meals). Any overseer puts a nursery or dormitory on
    `wants` when children ≥ 4. A Cautious overseer forbids the cavern rite and burrows children
    during sieges (the existing alert burrow).
  - The Wedding festival counts as Revel progress.
  - On screen: the sheet shows siblings, guardian, "Named for …", the heirloom and the rite; the
    family lines in ui.rs move to `ui_society::sheet_extra` in C00. The chronicle gets wedding and
    birth lines. O06's family tree screen reads `kin_of`.
- **Measure:** `tally("wedding")`, `("named_for_hero")`, `("adoption")`, `("rite_pass")`,
  `("rite_fail")`. Child deaths from the rite or wandering ≤ 1 per 25 forts.
- **Risks / open questions:** babies tied to their mothers can be dragged into fights; that is
  DF's famous "baby as weapon" and is fine. Inheritance re-pointing rooms must respect
  player-fixed rooms (`apply_zone_beds`).
- **Parallel work:**
  - **Writes:** family.rs (`families_daily` wedding and births, `new_year` rite), saga.rs (new),
    festivals.rs (Wedding variant and couple field: one match arm plus the guest filter in
    `start_festival`), and dwarf.rs `new_child` (the heredity call, already done in C00).
  - **Reads / depends on:** C00 (`kin`, `Saga`, `Errand`, `FestivalKind::Wedding`,
    `child_tick`/`on_arrival` hooks); C03 brawl optional (a stub until merged).
  - **Hot spots:** festivals.rs `start_festival`, which C04 also touches (`festival_tick` crowd
    hook). They are different functions; merge by function.
  - **Agent size:** P3: steps 1–5 (2.5 sessions) and P3b: steps 6–10 (2.5 sessions). They can be
    one agent serially, or split across wave 1 and wave 2.

---

### C06 The council  (effort: 6 sessions; risk: med)

- **Design** (council.rs holds mandates after the C00 split, plus `Council`):
  - **Overseer answers mandates (NOB-01; shares ECO-35 with O08):**
    - `issue_mandate` also pushes `Lack::Mandate` into troubles. `attention` gets the arm `(_,
      Lack::Mandate) => 1.2`.
    - `fix()` arm → `council::ov_mandate(o)`. By trait: Stubborn ignores it 60% of the time
      (note: "Grudgingly, it let the mayor's mandate lie"); Vain over-delivers (orders count + 2);
      the rest order the count.
    - The order goes through `mandate_recipe(kind, mat) -> Option<Recipe>` (Craft → StoneCraft,
      Table → StoneTable/WoodTable by mat, Chair, Door, Barrel, Bed) and the existing work orders
      (`add_order` at a matching shop, or an `o.boosts` entry).
    - A mandate it ignored and that went unmet costs favour.
  - **Mandate flavour (NOB-39):** `Mandate.reason: String` and `Mandate.penalty: Penalty {Blame,
    Jail, Fine}` (`#[serde(default)]`). The reason comes from the noble's facets ("for the
    feast-hall, to impress the Count"). `describe()` adds "or someone will answer for it", and
    the penalty is applied in `check_mandate` (Jail sets a crime with the jail sentence through
    C07's graded punishments, else beating).
  - **Nobles play favourites (NOB-07):** 30% of mandates are for goods made by the noble's best
    friend's labor (the friend gets pride, and others with the labor envy). Or a **ban**:
    `council.bans: Vec<Ban { recipe, noble, until }>` on a rival's craft. orders.rs
    `manage_orders` skips banned recipes (1 check), and the rival gets a slight (MND-17) and
    regard −30 toward the noble.
  - **Office envy (NOB-40):** in `nobles_daily`, each noble compares their office or bedroom value
    with other nobles of equal or lower rank. Worse: "is envious of X's office" (+12), favour −5.
    It feeds the existing demands.
  - **Export bans (NOB-43):** `council.export_bans: Vec<(ItemKind, noble, until)>` issued
    seasonally by a Greedy-facet noble (high artistry) on goods the fort makes. `ov_trade` skips
    banned items in its offer. A player trade of a banned item (trade.rs: 1 check at offer
    confirm) angers the noble (+20, favour −15) and a crime is charged to the broker (Disobedience).
  - **Choose your baron, or refuse (NOB-11; B02's OVR-09 reuses this):** `titles_daily` no longer
    names the baron directly. It creates `Petition { kind: Barony { candidate }, from: civ (0) }`
    with the 3 best candidates in `council.barony_choices`. `ov_petitions` decides by trait:
    - Vain: the one with the best room;
    - Sentimental: the most-liked;
    - Greedy: the richest broker;
    - Stubborn: refuses 40% of the time (`council.crown_snub = true`, the next caravan brings 25%
      fewer goods and a message);
    - otherwise the most sociable, as now.
    The player picks on the Nobles screen with 1–3/r. The noble's favour starts high toward the
    overseer that chose them.
  - **The council lobbies (NOB-02, OVR-03; one mechanism):**
    - Every noble and post holder has an **Agenda**, `agenda(holder) -> Ambition`: Commander →
      Muster or Fortify, Broker → Prosper, Manager → Provision, ChiefMedic → Provision (a
      hospital), Sheriff → Fortify, Mayor → Revel, titled → Architect.
    - The new Sense::**Lobby** = `Σ pressure / n`, where pressure = `(stress/400 +
      (favour<0)*0.5)`. In `brain()`'s `social_links`, each ambition gets `+0.8 * its lobbyists'
      pressure`.
    - Trait response: Stubborn ×0.3, and each season it ignores the top lobbyist, that noble's
      favour drops by 10 and they gain stress ("unrest"). Vain ×1.3 (wants approval). Cautious
      ×1 for Fortify and ×0.5 for the rest.
    - Favour: `council.favour: Vec<(u32, i32)>`, −100..100; it goes up when their agenda becomes
      the ambition or a mandate is met.
  - **Spin (NOB-09, shared with O07's OVR-05):** `write_chronicle` passes each siege line through
    `spin(line, siege)`. If the overseer is Vain, or the top noble's favour is < 0 and they are
    vain (artistry > 25), a siege with `dwarves_lost > foes_slain` or "the fortress fell" is
    written as "a glorious stand" or "the foe was driven back". Every dwarf who witnessed a death
    in that siege (Happening witnesses) gets "scoffed at the chronicle's lies" (+8), trust −10
    toward the overseer's appointees, and `tally("spin")`.
  - **Elections and complaints (NOB-45):** yearly at new year once pop ≥ 20, the mayor is chosen
    by the sum of regard (C03's `compat` helps) minus resentments (MND-17). A new mayor wipes the
    old mayor's mandates and bans and gets an announcement and legend. The loser gets +20 and a
    slight.
  - **A council's counsel (OVR-22):** `council_panel(g)` on the Nobles screen. Each advisor (post
    holder or noble) says one line from their agenda plus concrete state ("Commander: we have 3
    iron breastplates in stock and no one wears them", "Broker: the caravan comes in 12 days; we
    have ☼400 of crafts") and their favour. The overseer's current ambition is shown against
    each ("heeded" or "ignored"). It reads only existing state.
  - **Noble displeasure (OVR-29):** part of Lobby's pressure (unmet demands, open mandates, office
    envy, bans broken). No separate sense.
  - **Petitioners at the leader's door (OVR-30; B02's OVR-11 reuses this):**
    - Each day a dwarf with a grievance may walk to the mayor's or leader's office (or their bed if
      there's no office) with `Errand::Petition`. Grievances are: unpunished crime against them,
      room envy, a shunning they dispute, a theft. On arrival they make
      `PetitionKind::Complaint { against, about }`.
    - `ov_petitions` rules by trait. Sentimental: give them the better room or a feast. Cautious:
      refer it to the sheriff (C07 opens a case). Stubborn: dismiss. Greedy: fine the other
      party (an item).
    - Moods move by the ruling: granted −15 and favour +, dismissed +10 and a slight. The
      announcement reads "The mayor heard Sorli's complaint against Kelda and dismissed it".
- **Steps:**
  1. Mandates as troubles, and the overseer answering them by trait. · council.rs, troubles.rs (1
     variant arm and 1 `fix` arm) · **"Done when" test**
     `council::tests::a_mandate_is_met_or_refused_and_refusal_costs_favour`: a Stubborn overseer
     with a forced rng refuses, the deadline passes, the noble's favour falls; a Vain overseer
     queues orders.
  2. Mandate flavour and penalties. · test `council::tests::a_mandate_names_its_penalty`.
  3. Favourites and bans. · council.rs, orders.rs (1 check) · test
     `council::tests::a_noble_bans_a_rivals_craft`.
  4. Office envy. · test `council::tests::a_noble_envies_a_better_office`.
  5. Export bans. · council.rs, trade.rs (2 checks) · test
     `council::tests::the_overseer_wont_sell_banned_goods`.
  6. Barony choice or refusal. · positions.rs `titles_daily`, council.rs, ui (Nobles) · test
     `council::tests::a_stubborn_overseer_refuses_the_barony`.
  7. The Lobby sense and agendas (NOB-02, OVR-03, OVR-29). · council.rs, society.rs
     `social_links` · test `council::tests::the_commander_lobbies_for_walls`: after a siege, with
     a stressed commander, Fortify's odds rise more for a Vain overseer than for a Stubborn one.
  8. Elections. · test `council::tests::a_lost_election_wipes_the_old_mandates`.
  9. Spin. · chronicle.rs `write_chronicle` (1 call), council.rs `spin` · test
     `council::tests::a_vain_overseer_spins_a_rout_and_the_survivors_scoff`.
  10. Petitioners. · council.rs, errands.rs `Errand::Petition` · test
      `council::tests::a_complaint_is_heard_and_moods_move_by_the_ruling`.
  11. The counsel panel. · ui_society.rs `council_panel` · test (pure)
      `ui_society::tests::the_council_panel_lists_every_advisor`.
- **Overseer & screen:** this item is the overseer's social half: the Lobby sense in the brain
  screen (with its links shown like the other senses), the mandate decisions in its notes, voiced
  by trait, and petitions ruled in announcements. On the Nobles screen: favour bars, open
  petitions (y/n for the player when not autoplaying), barony choices and bans.
- **Measure:** `tally("mandate_met")`, `("mandate_ignored")`, `("mandate_failed")` (target: met
  ≥ 50% of mandates for non-Stubborn overseers), `("petition_granted")` and `("_refused")`, and
  survival unchanged (lobbying should not drag Warlords off Muster). Compare ambition mixes by
  style before and after.
- **Risks / open questions:**
  - Lobbying could homogenise overseers. Cap Lobby's link weight at 0.8, below Temper (traits)
    and Leaning (style).
  - **Decision for the user:** when the player (not the overseer) runs the fort, should petitions
    and the barony wait for the player, pausing the game, or default after a day?
  - The crown snub needs a caravan hook (`caravan_season`, 1 line).
- **Parallel work:**
  - **Writes:** council.rs (most of it), troubles.rs (`Lack::Mandate` arms), orders.rs
    `manage_orders` (1 check), trade.rs offer checks, positions.rs `titles_daily`, chronicle.rs
    `write_chronicle` (1 call), ui_society.rs `council_panel`, and app.rs Nobles keys.
  - **Reads / depends on:** C00 (`Council`, `Petition`, `Sense::Lobby`, `social_links`, `Errand`)
    and C02's `resents` (for elections, stubbed).
  - **Hot spots:** troubles.rs `fix` (C06 is the only M3 user), app.rs Nobles mode (1 block), and
    `brain()`, via the C00 hook only.
  - **Agent size:** P7 = steps 1–5 (wave 1, 2.5 sessions); P13 = steps 6–11 (wave 2, 3.5 sessions).

---

### C07 Crime and punishment, for real  (effort: 6 sessions; risk: med-high)

- **Design** (justice.rs grows. `Crime` gains, all `#[serde(default)]`: `known: bool`
  (default true for old records), `reported: bool` (default true), `witnesses: Vec<u32>`, `at:
  Option<Pos>`, `item: Option<u32>`, `accused: Option<u32>`, `punishment: Option<Punishment>`.
  New `commit_crime(kind, culprit, victim, what, at, item) -> usize` creates an unreported crime
  whose witnesses come from `happen()`. The existing `report_crime` stays: known and reported.):
  - **Witnesses report crimes (NOB-05):**
    - Each witness of an unreported crime (not the culprit, not covering for them: trust ≥ 40 or
      fear ≥ 40 toward the culprit, or the culprit is kin) takes `Errand::Report { crime }` to
      the sheriff (if none, the mayor or leader). On arrival, `reported = true` and `known =
      true`.
    - If the sheriff is the culprit, they "dismiss" it (a gossip line; trust in the sheriff falls).
    - Finding a body with wounds (C02 `found_body`) also opens a Murder case with an unknown
      culprit when the killer wasn't a creature.
    - `find_punishment` only takes reported, known crimes.
  - **Graded punishments (NOB-04):**
    - `Punishment { Fine, Shaving, Stocks, Whipping, Beating, Jail(days), Hammering, Banishment,
      Death }`, chosen by `sentence(crime list, sheriff facets, jail state)`:
      - severity = Σ kind weight (Vandalism 1, Disobedience 1, Drunkenness 1, Theft 2, Assault 2,
        Bribery 2, Escape 3, Desecration 3, Murder 6);
      - the sheriff's temper raises it, and a Sentimental overseer's policy lowers it.
    - Effects:
      - **Fine:** the culprit's best owned or carried item goes to stock.
      - **Shaving:** shame (+20), respect −15 from everyone, nickname candidate "Bare-Chin".
      - **Stocks:** an `Errand::Stocks` at a `BuildingKind::Stocks` (or the meeting area) for a
        day; a crowd forms (C04) and jeers by temper.
      - **Whipping and beating:** armour is unequipped and dropped first (`release armour` loop
        as in `go_insane`), then non-edged wounds.
      - **Jail** as now.
  - **Banishment and expulsion (NOB-03; B02's OVR-08 reuses this):**
    - At severity ≥ 6 or a third conviction, or by the overseer's ruling, the dwarf is banished:
      they walk to the map edge (`Errand::Leave`) and are moved to `Game.exiles`, like
      expeditions' `members` (removed from `dwarves`, not dead, so the death counts are
      unchanged). Positions, rooms, squad and pets are cleared.
    - Kin may follow: the spouse 50%, children under 12 always with a following parent.
    - Friends (regard ≥ 50) resent the sheriff or overseer. Legend and spotlight.
    - Exiles never return as migrants (migrant names avoid `exiles` names). B04's "exile returns"
      reads this list later.
  - **Missing, then dead (NOB-06):** implemented in C02 step 1. Here only the sheriff's part: a
    missing dwarf found with wounds opens a case.
  - **Theft as a citizen crime (NOB-20):**
    - In `justice_daily`, a citizen covets: stress > 120, or artistry > 25 and a free item of
      `likes_mat` worth ≥ 30, or a gambling debt (C09); diligence < 10 makes it likelier. They
      take it to their room (`Errand::Steal { item }`; on arrival the item's pos is in their room,
      `forbidden = true` so haulers leave it).
    - Witnesses come from `happen(Theft)`.
    - Visitors (C09 guests) steal from the depot too (visitors.rs: one branch in `visitor_think`).
    - **Missing items are noticed:** the bookkeeper's count (or daily without one) compares the
      count of items worth ≥ 30 with yesterday's minus known uses. A gap opens Theft with an
      unknown culprit and an unknown item, "Someone has stolen a gold goblet".
    - Totals go in the Society tab and the chronicle ("3 thefts, 1 solved").
    - The skulker (combat.rs `skulker_think`) gets a `commit_crime(Theft)` record when seen
      stealing.
  - **Unsolved crimes and accusations (NOB-19):** an unknown-culprit crime more than 3 days old
    gets an accusation from the sheriff (or from the overseer, `ov_justice`, if there's no sheriff):
    - `suspect_score(d) = 40*true witness report + suspicion(d) (C03) + 15*prior convictions +
      false testimony`;
    - false testimony: a witness or any dwarf who `resents()` someone names them (+30), and the
      real culprit, if questioned, names their worst enemy (+20);
    - the highest score is accused (`accused = Some`) and punished like a known culprit. A
      Cautious overseer needs score ≥ 50, otherwise it stays unsolved.
    - **Wrong accusation:** if accused ≠ culprit, the accused gets an injustice core memory
      (+40, bitterness) and kin are furious with the sheriff. If the truth comes out later (the
      stolen item is found in the real thief's room by a hauler), the sheriff loses respect,
      the false witness gets "Forked-Tongue" and the real thief is charged.
  - **Bribery (NOB-10):** when the sheriff arrives (`do_punish`), a culprit with an item worth ≥
    50 offers it. A sheriff with diligence < 0 and trust toward the culprit ≥ 0 accepts at 50%:
    charges dropped, the item moves, and a `Bribery` crime is committed by both with any
    witnesses. "Some can't be bought": diligence > 30 never accepts, and reports the attempt.
  - **Prison life (NOB-52):**
    - `serve_sentence` keeps prisoners in the jail (no leaving to eat). Instead `justice_daily`
      posts `Errand::Feed { prisoner }` for a hauler (a food and drink item brought to them).
    - Kin visit (`Errand::Visit`; −10 both).
    - Escape: if the jail zone has a door tile without a lock (any non-door boundary walkable) or
      the guard (sheriff) is away for 2 days, 5% a day. Escape is a crime, and the sheriff hunts
      (the existing `find_punishment`).
    - `BuildingKind::Stocks` (pre-added in C00) doubles as chains in a cell: a chained prisoner
      can't escape.
  - **Prison unrest (NOB-14):** capacity = jail tiles / 4. Over capacity, prisoners get +10 a day;
    at 2× capacity a riot: a brawl among the prisoners (C03, `BrawlCause::Prison`), then a
    breakout of all prisoners (escape crimes). Nobles stop jailing when full: `sentence` falls
    back to beating, whipping or the stocks, and the mayor gets "displeased at the overflowing
    jail" (Lobby).
  - **Hammerer and executions (NOB-38):**
    - `Position::Hammerer` (pre-added) is appointed by `ov_posts` at pop ≥ 30 if the overseer is
      not Sentimental. The suitability is temper + fight.
    - Hammering is a heavy blunt beating, fear +30 from everyone toward the hammerer.
    - Death is only for Murder with severity ≥ 6, needs a hammerer, and only if the overseer
      allows it (Cautious, Stubborn or Vain by default). It is carried out at the meeting area
      before a crowd and recorded as cause "executed" in annals (`cause_of`: add "executed").
  - **Public punishment (NOB-37):** stocks, whipping, hammering and executions gather a crowd
    (break_until like a festival, DAY/10). Reaction by values: sociable and temper < 0 means
    horror (+15) and trust in the sheriff −; temper > 20 cheers (−8); victims of the culprit get
    relief (−20). `crowd_feel` spreads it (C04).
  - **Witch hunts and scapegoats (NOB-22):** after ≥ 2 unsolved deaths or murders, or 3 unsolved
    thefts, in a season, `justice_daily` raises a mob. The top `suspicion` dwarf (outsider,
    shunned, cursed by a god (C10), a misfit) is blamed by the most anxious dwarves. The overseer
    rules (`ov_justice`): Cautious or Pious allow it (beating and shunning, or banishment if a
    Pious overseer and the target is cursed); Sentimental or Cheerful stop it (the mob's
    stress +). Rightly or not, it goes into a chronicle line and a spotlight.
- **Steps:**
  1. Crime fields, `commit_crime`, witnesses reporting (`Errand::Report`), cover-ups. · justice.rs,
     errands.rs · test `justice::tests::a_witness_walks_to_the_sheriff`.
  2. Theft by citizens, missing-item noticing, skulker record. · justice.rs, combat.rs (1 line) ·
     test `justice::tests::a_covetous_dwarf_steals_and_the_count_notices`.
  3. Accusations, false witness, wrong verdicts and the truth coming out. · **"Done when" test**
     `justice::tests::a_theft_with_one_witness_leads_to_an_accusation_that_is_sometimes_wrong`:
     over 20 seeds of a staged theft with one witness who resents a bystander, at least one
     verdict is right and at least one is wrong.
  4. Graded punishments (with armour removal). · test
     `justice::tests::the_punishment_fits_the_crime_and_armour_comes_off`.
  5. Bribery. · test `justice::tests::a_lazy_sheriff_takes_a_bribe_and_a_diligent_one_reports_it`.
  6. Prison life and escapes. · test `justice::tests::prisoners_are_fed_in_their_cells`.
  7. Prison unrest. · test `justice::tests::an_overcrowded_jail_riots`.
  8. Banishment and kin leaving, exiles. · test
     `justice::tests::a_banished_murderer_leaves_with_their_spouse`.
  9. Hammerer and executions. · positions.rs (suitability arm), overseer.rs `ov_posts` (1 row),
     annals.rs `cause_of` · test `justice::tests::a_murderer_is_hammered_by_the_hammerer`.
  10. Public punishment and crowds. · test
      `justice::tests::the_crowd_jeers_or_turns_away_by_temper`.
  11. Witch hunts. · test `justice::tests::unsolved_deaths_raise_a_mob_against_the_outsider`.
- **Overseer & screen:**
  - `ov_justice`: who accuses without a sheriff; whether to allow executions, mob justice and
    banishment (by trait); jail building when the jail is full (`wants.push(RoomUse::Jail)`;
    Cautious first).
  - The Discord sense (C03) adds unsolved cases.
  - On screen: a "Justice" section on the Society tab with open cases (unknown or accused),
    convictions and wrong ones once discovered. The sheet shows the record. Announcements and
    spotlights for verdicts, escapes, riots and executions. The chronicle gets "N crimes, M
    solved".
- **Measure:** `tally("crime_reported")`, `("crime_unsolved")`, `("wrong_accusation")` (target:
  10–30% of accusations), `("banished")`, `("executed")`, `("riot")`. Survival unchanged; banishments
  ≤ 0.5 per fort over 3 years.
- **Risks / open questions:**
  - Banishment and executions remove dwarves from fragile forts. Gate them on pop ≥ 20 for
    autoplay, and let the user decide whether the overseer may execute at all.
  - Missing-item noticing must not fire on consumption (meals eaten, bars smelted). Only count
    durable kinds (crafts, furniture items, weapons, armour, gems).
- **Parallel work:**
  - **Writes:** justice.rs (most), errands.rs (Report, Steal, Feed, Visit, Stocks, Leave),
    combat.rs `skulker_think` (1 line), visitors.rs `visitor_think` (1 branch), positions.rs
    (Hammerer suitability), overseer.rs `ov_posts` (1 row), and annals.rs `cause_of` (1 word).
  - **Reads / depends on:** C00 (`happen`, `Errand`, Crime variants, `exiles`); C03
    (`suspicion`, `tie`, brawls); C02 (`resents`, `GriefBook` finding bodies); C04 (`crowd_feel`)
    for steps 10–11.
  - **Hot spots:** overseer.rs `ov_posts`, 1 row (C09 adds the TavernKeeper row: an adjacent-line
    conflict, so pre-add both rows in C00 with `false` gates).
  - **Agent size:** P11 = steps 1–5 (wave 2, 3 sessions); P15 = steps 6–11 (wave 3, 3 sessions).

---

### C08 The bottle  (effort: 3 sessions; risk: med)

- **Design** (bottle.rs; `DrinkState { bac: u16, tolerance: u8, last_drink: u64, spree_until:
  u64, outs: u8 }`):
  - **Drunkenness as a level (HLT-04):**
    - `imbibe(d, at_festival)` is called from game.rs `Task::DrinkItem` (1 line) and festivals.rs
      `festival_drink` (1 line). It adds `140 − tolerance` to bac. Tolerance =
      `20*Drink-need strength + years drinking/10`, capped at 90.
    - bac decays 3 every 10 ticks (`bottle_tick`).
    - Levels (`Dwarf::tipsy()`):
      - **Merry ≥ 100:** −5 spirits once.
      - **Rash ≥ 300:** effective temper +25 for brawl rolls.
      - **Stumbling ≥ 500:** `move_cd` +1, trip checks.
      - **Sick ≥ 700:** vomits: a spill (below), a work pause.
      - **Out ≥ 850:** falls asleep on the spot (`asleep = true`, `Task::Idle`), `outs += 1`.
      - **Poisoned > 1000:** hurt each 100 ticks; death 10% a check (cause "drank themselves to
        death").
    - Heavy drinkers come from personality: Drink need 3 and anxiety > 20.
  - **Drink withdrawal (HLT-09):** dwarves with Drink-need strength ≥ 2, more than 3 days since
    `last_drink` of alcohol (water doesn't count):
    - "shaky": focus −10 through a `focus_penalty` hook (C00 `Dwarf::focus` calls
      `bottle::focus_mod(self)`);
    - "irritable": temper +15 for arguments;
    - at day 7, "sick with want of drink": work at 70%, +4 a day.
    One drink clears it ("steadied their hands with a drink at last").
  - **Drinking yourself to death (HLT-10):** level-headed dwarves (anxiety < 0 and diligence > 0)
    stop at Merry. At festivals, the drinking contest (festivals.rs `mugs`) runs to Out for the
    rash. The tavern keeper (C09) refuses anyone Stumbling or worse. A poisoning death is in the
    batch causes.
  - **Tripping (HLT-24):** game.rs `travel` gets a single hook `if self.maybe_trip(d) { return
    Travel::Moving }`, checked only when bac ≥ 500 or the tile is mud or rubble. 4% a step: lose
    the step, bruise 1. Next to an open edge (a down tile with no floor), fall one level ("fell
    down the stairs drunk", the "Tumbler" nickname). The cost is one branch on a field read for
    sober dwarves.
  - **Contact poisoning by grooming (HLT-27):** vomit, a dropped mug in a brawl, or a burst barrel
    (fire) leaves `Game.drink.spills: Vec<(Pos, u64)>` (dropped after 2 days). A cat or dog
    walking over one grooms and gets `Creature.tipsy` (pre-added in C00) +300, wobbling (move
    delay ×2) for a while. Gossip: "Soot the cat is staggering drunk after licking up spilled
    ale". The same path is ready for venom later (D05).
  - **Drowning sorrows (MND-08):** in `bottle_tick`, stress > 250 and Drink strength ≥ 2, at most
    once a season: `spree_until = tick + DAY/2`. The leisure chain's `spree` drinks repeatedly at
    the tavern or barrels; each drink is −25 stress, and bac climbs. When Rash and anyone with a
    grudge is within 5, a brawl (`BrawlCause::Drunk`) starts at 30%; if Out, they sleep it off. A
    gossip line and a memory.
  - Drunkenness at the wrong time (a soldier on alert, or working a lever or forge while
    Stumbling) commits the Drunkenness crime (C07).
- **Steps:**
  1. Levels and `imbibe`. · bottle.rs, game.rs (1 line), festivals.rs (1 line) · test
     `bottle::tests::drink_by_drink_a_dwarf_gets_merry_then_rash_then_out`.
  2. Withdrawal. · bottle.rs, the `focus_mod` hook · test
     `bottle::tests::a_dry_fort_gets_the_shakes`.
  3. Poisoning and stopping by personality. · test
     `bottle::tests::the_level_headed_stop_and_the_rash_can_drink_themselves_to_death`.
  4. Tripping. · game.rs `travel` (1 line), bottle.rs · test
     `bottle::tests::a_stumbling_drunk_trips_and_may_fall_downstairs`.
  5. Spills and the drunk cat. · bottle.rs, combat.rs creature move delay (1 line) · test
     `bottle::tests::a_cat_licks_up_spilled_ale`.
  6. Drowning sorrows. · bottle.rs, leisure.rs `spree` · **"Done when" test**
     `bottle::tests::a_festival_leaves_drunks_and_one_brawl_and_dryness_shows`: a Harvest festival
     with 40 drinks and two rash rivals ends with ≥ 2 dwarves at Merry or above and one brawl; a
     second game with no drink for 8 days shows "shaky" thoughts.
- **Overseer & screen:**
  - The Thirst sense already makes the overseer brew. New: a trouble `Lack::Drink` fires earlier
    when withdrawal is showing (reusing the existing Lack).
  - Cheerful overseers raise brew targets (`ov_orders`, existing Revel boost).
  - Cautious overseers ban drinking during an alert: soldiers skip `Task::DrinkItem` of alcohol
    while the alert is on, a filter in `choose_task`'s drink branch read from a council flag.
  - On screen: the sheet shows "Merry / Rash / Stumbling / Out cold" next to the mood, and
    "shaky"; drunk dwarves are drawn with a reeling glyph colour (ui.rs map: 1 colour tweak behind
    `tipsy()`). Gossip and nicknames ("Barrel").
- **Measure:** `tally("drunk_out")`, `("drunk_brawl")`, `("withdrawal")`, deaths "drank
  themselves to death" (target ≤ 0.5% of deaths), and survival unchanged (tripping must not
  add falls in sieges: check deaths by "crushed" and "fell").
- **Risks / open questions:**
  - Rash soldiers on watch. Alcohol in sieges could matter (DF dwarves fight drunk). Default: no
    effect on combat skill, only the brawl temper.
  - The travel hook is the per-tick hot path. Measure `one_year_timing`.
- **Parallel work:**
  - **Writes:** bottle.rs (new), game.rs (`Task::DrinkItem` 1 line, `travel` 1 line),
    festivals.rs `festival_drink` (1 line), leisure.rs (`spree`, registered in the C00 chain),
    combat.rs creature move delay (1 line), and ui.rs map tint (1 line).
  - **Reads / depends on:** C00 (`DrinkState`, `focus_mod` hook, `Creature.tipsy`, leisure chain)
    and C03 brawls for step 6 (a stub until merged).
  - **Hot spots:** game.rs `travel` and `do_task` (2 lines) and ui.rs (1 line).
  - **Agent size:** one package, P4 (wave 1; step 6's brawl part can land in wave 2).

---

### C09 The tavern night  (effort: 5 sessions; risk: med)

- **Design** (tavern.rs; `TavernBook { guests: Vec<Guest>, bets: Vec<Bet>, renown: i32, keeper:
  Option<u32> }`):
  - **Drunks and bar brawls (SOC-05):** in a tavern zone, Rash drunks with any regard ≤ −20
    pair start brawls (C03) at 10% a check. Those Out are carried to bed by the keeper or a kind
    dwarf (C04 `Errand::Escort`, for an unconscious-like state: the escort walks and the drunk's
    pos follows). Drinking at the wrong time is a crime (C08).
  - **Gambling and debt (SOC-06):**
    - Idle dwarves in the tavern play dice: a pair with sociability > 0, stakes of 1–3 "marks"
      (an abstract `social.debts: Vec<(creditor, marks)>`).
    - Bets are placed on duels (`fight_duel`), arm-wrestling and arena fights (below).
    - A debt of ≥ 5 marks: the creditor pesters (regard −5 a week), or the debtor steals to pay
      (C07 theft motive) or hands over an item (the debt is cleared).
    - "Lucky" and "Cursed-Dice" nicknames.
  - **Tavern keepers and performers (SOC-30; F07 performers reuse this):**
    - `Position::TavernKeeper` (pre-added) is appointed by `ov_posts` when a tavern zone exists
      and pop ≥ 10; suitability is sociability + temper calm.
    - The keeper serves: `Errand::Serve` brings a drink item to a dwarf in the tavern (they drink
      without walking to the stockpile), and refuses those Stumbling or worse.
    - Resident performers are citizens with Party need ≥ 2 and sociability > 25 (or C09 resident
      visitors). Every evening (DAY*3/4) one performs dance, music or a story (story text from
      `annals.deaths` / legends / C09 tall tales).
    - `tavern.liveliness` (performances and patrons this day) multiplies the Party and Socialize
      satisfaction in `needs_tick`'s zone bonus (needs.rs `zone_need` stays; the multiplier comes
      from `tavern::bonus(g, pos)`).
  - **Tall tales and liars (SOC-19):** in tavern chats (the C04 chat type "story"), a dwarf with
    kills, an expedition or an epithet tells a tale. Embellishment = `max(0, artistry/10 +
    rng)`: "slew 7 goblins at the gate" (real: `kills`, or the happening count). Listeners: respect
    +5. A listener who witnessed the real happening (Happening witnesses) catches the lie at
    embellishment ≥ 3: trust −15, gossip, and after 3 exposures the nickname "Tall-Tale" and a
    legend line "known as a liar".
  - **Visitors bring old feuds (SOC-14):**
    - `spawn_visitor` (visitors.rs, 1 branch) has a 20% chance of giving the guest
      `Creature.feud_with: Option<u32>` (pre-added in C00): a citizen over 40, not born here
      ("Ostra Ashhelm crossed this wanderer's kin in the hill wars"). The guest doesn't know the
      citizen is here.
    - When both are in the tavern within 4 tiles, the guest attacks (Fists) or insults, then
      attacks (existing visitor→creature combat with `Task::Attack` by the citizen; the guest is
      hostile only to them).
    - It is a crime by the guest (`commit_crime(Assault)`; the sheriff may jail or banish a
      visitor). A rare murder is a legend.
  - **Taverns with paying guests (SOC-31):**
    - New visitor kinds MERCHANT, MERCENARY and ADVENTURER (consts in visitors.rs, like BARD).
      They come at a rate rising with `tavern.renown`, rent a bed in a Bedroom zone flagged
      `Zone.guest: bool` (pre-added) and pay on leaving (spawn a trade good or coins-as-crafts at
      the depot).
    - Renown goes up with liveliness and performances and down with brawls, thefts and murders of
      guests.
    - Their crimes feed C07: guests steal from the depot (C07 step 2 hook).
  - **Residents and citizens (SOC-32):** a performer or mercenary staying > 1 season files
    `PetitionKind::Residency { visitor }`. If granted, they stay indefinitely (a resident; the
    visitor's `leave_tick` extends). After a year, `PetitionKind::Citizenship`. Granted dwarves
    become citizens through the existing `warrior_joins` conversion; other races stay residents.
    The overseer answers by trait: Cautious refuses strangers, Cheerful welcomes, Greedy grants
    paying mercenaries. A refused resident leaves bitter (renown −).
  - **Arena and fight pit (SOC-26):**
    - `ZoneKind::Arena` (pre-added). A creature caught in a cage trap (traps.rs keeps the cage
      contents; check `Building` occupant) can be released into an Arena zone by the overseer
      (Bold or Vain) or the player (`Errand::Release`).
    - A champion (the best soldier or a volunteer by temper) fights it before a crowd: break_until
      for spectators, C04 crowd moods, bets (SOC-06) with the keeper as bookmaker.
    - A win gives fame (epithet "Pitfighter", respect); death is losing, and fun. It is never
      during an alert.
- **Steps:**
  1. Keeper post, serving and refusing, liveliness bonus. · tavern.rs, positions.rs (duty and
     suitability arms), errands.rs `Errand::Serve`, needs.rs (1 multiplier call) · test
     `tavern::tests::the_keeper_serves_and_refuses_the_drunk`.
  2. Bar brawls and carrying out the drunk. · test
     `tavern::tests::rash_drunks_with_a_grudge_brawl_in_the_tavern`.
  3. Performers. · test `tavern::tests::a_performer_livens_the_tavern`.
  4. Gambling and debt. · tavern.rs, drama.rs `fight_duel` (bet settle, 1 call),
     festivals.rs `arm_wrestle` (1 call) · test `tavern::tests::a_heavy_loser_owes_and_resents`.
  5. Tall tales and liars. · test `tavern::tests::a_witness_catches_a_tall_tale`.
  6. Visitors bring old feuds. · visitors.rs `spawn_visitor` (1 branch) and `visitor_think` (1
     branch) · **"Done when" test**
     `tavern::tests::a_guest_with_a_feud_meets_their_enemy_and_a_fight_breaks_out`.
  7. Paying guests and renown. · visitors.rs (new kinds), tavern.rs · test
     `tavern::tests::guests_rent_rooms_and_pay`.
  8. Residents and citizens (petitions). · test
     `tavern::tests::a_cheerful_overseer_grants_residency`.
  9. Arena and fight pit. · tavern.rs, traps.rs (release a caged creature, 1 fn) · test
     `tavern::tests::a_caged_beast_fights_a_champion_before_a_crowd`.
- **Overseer & screen:**
  - `ov_tavern`: appoint a keeper; a Cheerful or Vain overseer with renown < 0 builds a bigger
    tavern (`wants`) and guest rooms; residency rulings; arena releases (Bold or Vain).
  - Renown feeds Revel's progress (`ambition_met`, 1 line).
  - On screen: the Society tab's "Tavern" section (keeper, tonight's performance, renown, guests,
    debts); announcements for performances, tall tales caught, guest feuds; the chronicle "The
    tavern of F became known across the hills".
- **Measure:** `tally("performance")`, `("guest")`, `("guest_feud")`, `("bet")`, `("tall_tale")`,
  `("arena_fight")`. Survival unchanged (arena deaths ≤ 1 per 25 forts).
- **Risks / open questions:**
  - Visitors are `Creature`s, not `Dwarf`s, so feuds and crimes with guests need creature-side
    handling. Keep guests out of the dwarf-only brawl engine; use the existing combat with fists.
  - The arena depends on cage traps actually holding creatures (traps.rs). If they don't, this
    step slips to D09 or a later item. **Decision for the user:** keep SOC-26 in M3, or defer it.
- **Parallel work:**
  - **Writes:** tavern.rs (new), visitors.rs (`spawn_visitor`, `visitor_think`, new kinds),
    positions.rs (TavernKeeper arms; the variant is pre-added), errands.rs (Serve, Release),
    needs.rs (1 call), drama.rs `fight_duel` (1 call), festivals.rs `arm_wrestle` (1 call),
    traps.rs (release fn), and overseer.rs `ov_posts` (row pre-added in C00).
  - **Reads / depends on:** C00 (`Petition`, `TavernBook`, `Zone.guest`, `Creature.feud_with`,
    `ZoneKind::Arena`), C08 (`tipsy`), C03 (brawls, debts in `Social`), C04 (crowds, stories), and
    C07 (`commit_crime`) for steps 6–7.
  - **Hot spots:** visitors.rs (only C09 in M3), drama.rs `fight_duel` (C03 edits
    `drama_daily`; a different function).
  - **Agent size:** P12 = steps 1–6 (wave 2, 3 sessions); P12b = steps 7–9 (wave 3, 2 sessions).

---

### C10 Gods of the mountain  (effort: 6 sessions; risk: med)

- **Design** (faith.rs; `Pantheon { gods: Vec<God>, priests: Vec<(u8, u32)>, curses: Vec<(u32,
  u8, u64)>, cults: Vec<(String, u32)> }`, `God { id: u8, name: String, spheres: [Sphere; 2],
  beast: Option<String> }`, `Faith { gods: Vec<(u8, u8)>, cursed: Option<u8>, prayed: u64 }`,
  `Zone.dedicated: Option<u8>`):
  - **Gods with spheres (MYT-10):** `Pantheon::for_civ(seed ^ civ)`, generated lazily on first
    use (`faith_daily` when `gods.is_empty()`): 5–7 gods, 2 spheres each from `Sphere {Stone,
    Fire, Craft, Ale, War, Death, Family, Love, Water, Caverns, Fortune, Night, Healing, Justice,
    Mountains, Beasts}`, with names from `overworld::person_name` plus a title ("Okab, the Anvil of
    Dawn"; original words).
    - Dwarves get 1–2 gods with devotion = 20 × Pray strength ± rng, picked by sphere and
      personality (high temper: War; high artistry: Craft; Drink need 3: Ale). Old saves are
      filled the first time `faith_daily` runs.
    - Dwarves may pray to any god at a temple; the bonus is for their own.
    - The legends screen gets a "Gods" tab: ui.rs `draw_legends` gets 1 tab through ui_society.
  - **Temples, priests and sermons (MYT-11):**
    - A temple zone can be dedicated (the zone menu `i`: a d key cycles gods; the overseer
      dedicates by majority faith).
    - `seek_need_zone` prefers a temple dedicated to one of the dwarf's gods (a filter in
      `wanted_zone` through `faith::temple_for(d)`). Praying there gives Pray +200 vs +120.
    - Once the temple's `room_at` value is ≥ 200 (or the zone has ≥ 2 built furniture and an
      Altar), a priest is appointed: the worshipper with the highest devotion + sociability, who is
      exempt from labour at that point (like a specialist).
    - A sermon every 5 days at DAY/3: worshippers within the zone get −15 (and stressed ones
      −25, through MND-59's "fragile can be helped"), crowd mood (C04), and 5% conversion of
      godless listeners.
  - **Temple design from the god (MYT-28):**
    - `sphere_taste(Sphere) -> (mats: &[Mat], items: &[BuildingKind], open_sky: bool)`: Fire likes
      obsidian or magma nearby; Stone likes granite and engraved walls (`detail_value`); Water
      wants a well or water within 5; Mountains or Night want an underground temple; Ale wants a
      still nearby; War wants weapon racks or an archery target; Death wants coffins or slabs.
    - `temple_fit(zone) -> 0..100` multiplies the sermon effect (0.5–1.5).
    - The overseer's `ov_fit` for `RoomUse::Temple` reads the god's taste (overseer.rs `ov_fit`: 1
      call to `faith::temple_furniture(god) -> Vec<BuildingKind>`).
  - **Religious petitions (MYT-12):**
    - ≥ 5 worshippers of a god with no dedicated temple → `PetitionKind::Temple { god }` from the
      most devout. Later, ≥ 12 → `Priesthood` (a "temple complex": a bigger room).
    - Granted and not built within a season: `Broken`, the worshippers +20, and favour − toward
      the overseer.
    - `ov_petitions` by trait: Pious always grants and pushes `RoomUse::Temple` to the front of
      `wants`; Greedy grants only if wealth/pop is high; Stubborn 50%; Cheerful grants if a tavern
      exists.
  - **Faith passed on (MYT-41):** at age 6 (`new_year`), a child takes the god with the highest
    combined devotion of the parents, at half the parent's devotion. Migrants keep theirs. Godless
    listeners convert through sermons (above) in waves.
  - **Cursing the gods (MYT-27):** on a Happening of a kin death, a lost siege or a fire, a dwarf
    with devotion < 30 and stress > 200 curses their god (a gossip line). Devout witnesses
    (devotion ≥ 60, same god) are scandalised: regard −20, +8, and possibly a C03 feud.
  - **Divination shrine (MYT-24):**
    - `BuildingKind::Altar` (pre-added; built from stone at a Mason) in a temple. A dwarf on a
      break casts lots there (`Errand::Lots`). A roll from an omen table gives:
      - good: −20 and Fortune +;
      - ill: +10;
      - "the god is silent";
      - 2%: a minor curse (bad luck for a season: mood penalty, more trips).
    - It is the groundwork for C09 dice.
  - **Desecration and divine curses (MYT-13; N03's "done when" reuses this):**
    - `tantrum_mischief` (psyche.rs) toppling a building inside a dedicated temple, or a C12 dark
      mood using a temple's altar, commits `CrimeKind::Desecration` and has a 25% chance of a
      curse: `faith.cursed = Some(god)`, "has been cursed by Okab" (spotlight, legend).
    - A breaking vow (C02 MND-05, sworn on a god in the curse table) has a 10% chance.
    - The curse's effect in M3: worshippers of that god shun the cursed (C03 shun score +5), bad
      luck, and no peace at sermons. The vampire and werebeast transformations wait for N01/N03,
      which plug into `faith.cursed`.
  - **Beast cults (MYT-14):**
    - When the same creature kind (from `annals.deaths` killers) has killed ≥ 4 citizens over ≥ 2
      seasons, or a named megabeast or figure has attacked twice, anxious dwarves (anxiety > 25)
      who witnessed a death to it gain devotion to a pseudo-god `God { beast: Some(kind),
      spheres: [Beasts, Night] }`. The superstition text drifts each season ("leave a bone at the
      cavern mouth").
    - With 3 members: a rare shrine (they claim a free Altar, or a statue-less corner of a
      meeting area). Sighting the beast: awe (−) for members, fear (+) for others.
    - Pious overseers tolerate cults; Cautious ones forbid the shrine (the Altar is removed or
      unclaimed).
  - **Answered prayers (MYT-15):** desperate prayer (stress > 300 or a siege, in a temple of their
    god) with a sphere fitting the need, once per god per year at 3% a day:
    - Healing heals the worst wound;
    - War or Water: a storm on the besiegers (weather.rs `force_storm`: a 1-call hook; foes
      slowed);
    - Fortune: a relic (a free craft becomes an artifact-quality item labelled "relic of Okab");
    - Family: a lost child found;
    - otherwise a sign (all worshippers −30).
    A legend, a spotlight and `tally("miracle")`.
- **Steps:**
  1. Pantheon and personal faith (with fill-in for old saves), and the Gods legends tab. ·
     faith.rs, ui_society.rs · test `faith::tests::every_dwarf_with_a_need_to_pray_has_a_god`.
  2. Dedication, own-god preference, priests, sermons. · faith.rs, needs.rs `wanted_zone` (1
     filter call), app.rs zone menu (1 key) · test
     `faith::tests::worshippers_go_to_their_gods_temple_and_hear_sermons`.
  3. Religious petitions and broken promises. · **"Done when" test**
     `faith::tests::a_temple_draws_its_worshippers_and_a_pious_overseer_grants_the_petition`: 6
     worshippers of god 0 and no temple produce a Temple petition; a Pious overseer grants it and
     adds `RoomUse::Temple` to `wants`; once dedicated, worshippers of god 0 spend more time there
     than others.
  4. Temple design from the god. · faith.rs, overseer.rs `ov_fit` (1 call) · test
     `faith::tests::a_fire_gods_temple_wants_obsidian`.
  5. Faith passed on and conversion. · test `faith::tests::children_take_their_parents_god`.
  6. Cursing the gods. · test `faith::tests::the_devout_are_scandalised_by_a_curse`.
  7. The divination altar. · faith.rs, buildings.rs (Altar recipe and cost arms filled) · test
     `faith::tests::lots_cast_at_the_altar_give_omens`.
  8. Desecration and curses. · psyche.rs `tantrum_mischief` (1 check) · test
     `faith::tests::toppling_a_temple_altar_in_a_tantrum_may_bring_a_curse`.
  9. Beast cults. · test `faith::tests::a_beast_that_kills_four_gets_a_cult`.
  10. Answered prayers. · faith.rs, weather.rs (1 hook) · test
      `faith::tests::a_desperate_prayer_is_rarely_answered_by_a_healer_god`.
- **Overseer & screen:**
  - The new Sense::**Devotion** (open temple petitions, worshippers without a temple, a recent
    miracle) leans toward Revel and Architect. Pious ×2.
  - `ov_faith`: dedicate temples by majority, grant or refuse petitions, place the Altar, and
    tolerate or forbid cults.
  - On screen: the Gods legend tab, the sheet's "Worships Okab (devout)" line, the temple shown in
    the look panel with god and fit, sermons and miracles in announcements, curses in spotlights,
    and cults in the chronicle.
- **Measure:** `tally("temple_dedicated")`, `("sermon")`, `("petition_temple")`,
  `("petition_broken")`, `("miracle")` (≤ 1 per fort-year), `("curse")`. Revel ambition frequency
  for Pious overseers.
- **Risks / open questions:** the gods' names and titles must be original (CLAUDE.md). Generating
  per civ at runtime keeps overworld.rs untouched, but the gods aren't in world-gen legends until
  B14. **Decision for the user:** generate in world-gen now (overworld.rs, a bigger change) or
  lazily (recommended).
- **Parallel work:**
  - **Writes:** faith.rs (new), needs.rs `wanted_zone` (1 call), psyche.rs `tantrum_mischief` (1
    check; C12 also edits this function, so sequence C10 step 8 before C12), overseer.rs `ov_fit`
    (1 call), weather.rs (1 hook), buildings.rs (Altar arms), ui_society.rs, and app.rs zone menu
    (1 key).
  - **Reads / depends on:** C00 (`Faith`, `Pantheon`, `Zone.dedicated`, `Petition`,
    `Sense::Devotion`, `BuildingKind::Altar`) and C04 crowd (sermons, stubbed until merged).
  - **Hot spots:** overseer.rs `ov_fit` (1 line), app.rs (1 key).
  - **Agent size:** P5 = steps 1–5 (wave 1, 3 sessions); P14 = steps 6–10 (wave 2, 3 sessions).

---

### C11 Table and tankard  (effort: 3 sessions; risk: low-med)

- **Design** (kitchen.rs; `Kitchen { brews: Vec<NamedBrew>, dishes: Vec<Dish> }`):
  - **Named drinks and vintages (AGR-05):**
    - When `Recipe::Brew` completes (game.rs's craft completion calls `Recipe::produce`; a hook
      `kitchen::on_produced(&mut items, maker)`), a brewer at skill ≥ Expert, or quality ≥ 4,
      has a 25% chance to name the batch after the most recent weighty Happening ("Midwinter
      Rout Stout", "Hilda's Wedding Ale"). The `label` is set on all 4 drinks.
    - Named drinks age: `wear` counts days in a barrel (`rot_daily` hook: +1 a day for labelled
      drinks in a container). Older is better: the drink thought −15 → −25 at ≥ 1 year
      ("savoured a year-old Midwinter Rout Stout").
    - Collectors: a dwarf with `likes_food == Drink`; old saves get it from a new roll in `on_arrival`.
    - At festivals, a named cask is opened for a toast (all attendees −10, a chronicle line).
  - **Signature recipes (AGR-06):**
    - A cook at ≥ Master skill who cooks a meal at quality ≥ 4 creates the fort's signature dish
      (once per cook): `Dish { name, cook, mat, created }`, named from the ingredients and the
      cook ("Kolvar's Stoutcap Dumplings"). A legend line and a chronicle mention.
    - Later meals by that cook have a 30% chance of being the dish (label). Eating it gives −15
      "ate the famous …".
    - The civilization remembers it: `overworld.recipes: Vec<String>` (one `#[serde(default)]`
      field in overworld.rs), so later forts' migrants can know it.
    - Nobles crave dishes: a titled noble's demand can be "serve me Kolvar's dumplings" (a
      `Demand` variant through council.rs, met by the noble eating one).
  - **Spoilage by heat (AGR-31):**
    - `rots()` in miasma.rs is extended to food (Meat, Fish, Meal, Plant) not in a container (a
      barrel or pot protects). Rot days vary by place: outside in summer ×2; within 6 tiles of
      magma ×2; outside in winter, frozen, no rot. Food rots away at 20 days (no miasma below
      ground unless meat or fish).
    - Gentle on purpose. Survival can't suffer: forts store food in barrels (does the overseer
      order barrels? It does: `Recipe::Barrel` is in `ov_orders`).
  - **Fishery (AGR-20):** `BuildingKind::Fishery` (pre-added) and `Recipe::CleanFish` (raw fish →
    2 cleaned fish, `ItemKind::Fish` with a `label` "cleaned"). Cooking prefers cleaned fish.
    Eating raw uncleaned fish is +4 "ate raw fish" (it was −4). Raw fish rots slowly (40 days).
    The overseer builds one (`ov_rooms` workshops list) once fishing produces ≥ 5 fish.
- **Steps:**
  1. Spoilage by heat. · miasma.rs `rots`, `rot_daily` · test
     `kitchen::tests::meat_left_in_the_sun_rots_and_barrels_keep_it`.
  2. Fishery. · buildings.rs (Recipe::CleanFish arms), kitchen.rs, overseer.rs workshop list (1
     entry) · test `kitchen::tests::fish_are_cleaned_before_the_cook_takes_them`.
  3. Named drinks and vintages. · kitchen.rs, game.rs craft completion (1 hook) · test
     `kitchen::tests::an_expert_brewer_names_a_brew_after_the_victory`.
  4. Signature recipes. · kitchen.rs, overworld.rs (1 field), council.rs demand variant · **"Done
     when" test** `kitchen::tests::a_legendary_cooks_dish_is_named_and_in_the_chronicle`.
- **Overseer & screen:** the Fishery workshop goes in the plan. `ov_orders` keeps food in barrels
  (raises `Recipe::Barrel` when food outside containers > 20). A Cheerful or Vain overseer opens a
  named cask at festivals. On screen: item names in stocks, sheet thoughts, the legends and
  chronicle, and the Society tab "Fine fare" list (brews and dishes).
- **Measure:** `tally("named_brew")`, `("signature_dish")`, `("food_rotted")`. **Starvation
  deaths must not rise** compared with the previous wave's batch (the gate on AGR-31).
- **Risks / open questions:** spoilage could undo S05 (food reliability). The step ships with
  rates halved if the batch shows starvation up by more than 1 death per 25 forts.
- **Parallel work:**
  - **Writes:** kitchen.rs (new), miasma.rs (`rots`, `rot_daily`), buildings.rs (CleanFish recipe
    arms, Fishery arms), game.rs craft completion (1 hook), overseer.rs workshop list (1 entry),
    overworld.rs (1 field), and psyche.rs `eat_thoughts` (raw fish and dish thoughts).
  - **Reads / depends on:** C00 (`Kitchen`, `BuildingKind::Fishery`, `happen` for naming).
    Nothing from other items.
  - **Hot spots:** buildings.rs Recipe (C10's Altar is a BuildingKind, not a Recipe: no overlap),
    overseer.rs (1 line).
  - **Agent size:** one package, P6 (wave 1).

---

### C12 Breakdowns  (effort: 2 sessions; risk: med)

- **Design:**
  - **Tantrums with limits (MND-32):**
    - `tantrum` becomes stamina: it drains by 1 a tick as now plus 2 more if hurt (the body's
      pain or hp loss) or exhausted (`fatigue > TIRED`), so a beaten-up raging dwarf burns out fast.
    - `tantrum_mischief` (psyche.rs) breaks only what is within reach (already adjacent). It never
      breaks artifacts or the dwarf's own bed unless stress > 600 ("in the blackest mood, smashed
      their own bed"). It strikes friends only if stress > 500.
    - S09's berserk is the extreme end. A tantrum that reaches stress > 700 hands over to S09's
      berserk, if S09 exists.
  - **Darker moods (MND-14):** `Mood.kind: MoodKind {Fey, Secretive, Possessed, Fell, Macabre}`
    (pre-added).
    - `maybe_strange_mood` picks the kind by the candidate's state: stress > 300 → Fell 40% or
      Macabre 60%; stress > 150 → Macabre 30%; otherwise Fey or Secretive. **Any dwarf can be
      taken, not only the moody labors**: a dark mood uses Bone and Crafting if the dwarf has no
      moody labor.
    - **Macabre:** needs bones, a corpse or remains ("remains of X" from `rot_daily` counts); the
      artifact gets a grim name (`dark_artifact_name`: "The Weeping Marrow", original word lists).
    - **Fell:** first claims a workshop, then needs a victim. It walks to the nearest sleeping or
      lone dwarf (not kin) and kills them (`kill(.., "was murdered in a fell mood")`, Murder on
      record, witnesses), then makes the artifact from their bones (`Mood.items` gets the corpse
      after `rot`: immediate bones item spawned "bones of X").
    - **The fort is shaken:** Happening DarkArtifact. Everyone who sees or hears of it gets "was
      disturbed by X's macabre work" (+15, Disgust or Horror); the fell killer gets nothing
      (`hardened` → 100).
    - The overseer (`ov_breakdown`): Cautious or Sentimental bans the fell dwarf to the jail
      (C07 Murder case: a hammerer, or banishment); Vain keeps the artifact on display (the
      monument hall); Pious calls it desecration if made in a temple (C10).
- **Steps:**
  1. Tantrums with limits. · psyche.rs `tantrum_mischief`, game.rs `update_dwarf` tantrum drain
     (2 lines) · test `psyche_tests::a_hurt_raging_dwarf_tires_and_spares_their_bed`.
  2. Darker moods. · moods.rs (`start_mood` kind choice, `do_mood` Fell and Macabre branches),
     dwarf.rs-free (MoodKind pre-added) · **"Done when" test**
     `moods::tests::a_dwarf_in_a_dark_mood_makes_a_macabre_artifact_and_the_fort_is_shaken`: force
     a Macabre mood with bones in stock; the artifact has a dark name; ≥ half the fort has the
     "disturbed" thought.
- **Overseer & screen:** the troubles `MoodShop`, `Bone` and `Gem` already exist. Add `Lack::Bone`
  urgency for Macabre (`attention` ×1.6 like MoodShop). `ov_breakdown` as above. On screen: a
  spotlight for the dark mood and the artifact, the sheet shows the mood kind, and the chronicle
  title "The Year of the Weeping Marrow" (YearLog.artifact already titles years).
- **Measure:** `tally("dark_mood")`, `("fell_murder")` (≤ 1 per 25 fort-years), `("tantrum")`
  duration mean (should fall), `("bed_smashed")`.
- **Risks / open questions:** fell moods kill a citizen in small forts. Gate them on pop ≥ 15.
  This item depends on S09 (M1) owning the madness end, and on C01's spirits. C10 step 8 edits the
  same `tantrum_mischief`; do C12 after it.
- **Parallel work:**
  - **Writes:** psyche.rs `tantrum_mischief`, game.rs `update_dwarf` tantrum lines (2), moods.rs
    `start_mood` and `do_mood`.
  - **Reads / depends on:** C00 (`MoodKind`, `happen`), C01 (spirits, `hardened`), C07 (Murder
    case), C10 step 8 (merged first), and M1 S09.
  - **Hot spots:** game.rs `update_dwarf`, 2 lines.
  - **Agent size:** one package, P16 (wave 3).

---

## Interface contracts (built in C00, fixed for every package)

All new types derive `Clone, Debug, Default, Serialize, Deserialize` (and `PartialEq` where
tests compare). Every new field on an existing struct is `#[serde(default)]`.

```rust
// ---------------- src/game/memory.rs ----------------
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Emotion { #[default] None, Joy, Pride, Relief, Gratitude, Love, Awe,
                   Grief, Horror, Fear, Rage, Shame, Envy, Loneliness, Bitterness, Disgust }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum About { Dwarf(u32), Creature(u32), Foe(String), Item(u32), God(u8), Place(Pos) }

pub struct Feeling<'a> { pub text: &'a str, pub stress: i32, pub emotion: Emotion,
                         pub about: Option<About>, pub at: Option<Pos>, pub hap: Option<u32> }
impl<'a> Feeling<'a> {
    pub fn new(text: &'a str, stress: i32) -> Self;
    pub fn emo(self, e: Emotion) -> Self; pub fn about(self, a: About) -> Self;
    pub fn at(self, p: Pos) -> Self;      pub fn hap(self, id: u32) -> Self;
}
impl Dwarf {                                   // impl block lives in memory.rs
    pub fn feel(&mut self, f: Feeling, tick: u64);   // think() now = feel(Feeling::new(..))
    pub fn spirits_word(&self) -> &'static str;      // "Upset", "Cheerful", ...
}
// Thought (dwarf.rs) gains: #[serde(default)] emotion: Emotion, about: Option<About>, at: Option<Pos>

pub struct Memory { pub text: String, pub emotion: Emotion, pub about: Option<About>,
                    pub at: Option<Pos>, pub stress: i32, pub tick: u64, pub relived: u8,
                    pub hap: Option<u32> }
pub struct InnerLife { pub spirits: i32, pub short: Vec<Memory>, pub core: Vec<Memory>,
                       pub hardened: u8, pub deaths_seen: u16, pub outdoor_days: u16,
                       pub changes: Vec<(u64, String)>, pub life: Vec<(u64, String)> }
// Dwarf gains: #[serde(default)] pub inner: InnerLife

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HapKind { Death, Wounded, FoeSlain, SiegeBegan, SiegeBroken, SiegeLost, Brawl, Tantrum,
                   Crime, Theft, Punishment, Execution, Banishment, Wedding, Birth, ComingOfAge,
                   Artifact, DarkArtifact, Festival, Sermon, Miracle, Curse, Desecration, Fire,
                   CaveIn, Performance, TallTale, Arrival }
pub struct Happening { pub id: u32, pub kind: HapKind, pub tick: u64, pub at: Pos,
                       pub actor: Option<u32>, pub target: Option<u32>, pub foe: Option<String>,
                       pub weight: u8 /*0..100*/, pub text: String, pub witnesses: Vec<u32> }
impl Game {
    /// Records it (ring of 300, saved), fills witnesses (awake, alive, same z, Chebyshev ≤ 8 by
    /// default or `weight`-scaled up to 15), then calls every *_react stub in society.rs.
    /// NOTE: a dwarf taken out of `self.dwarves` for its turn is not in the scan; callers pass
    /// it as `extra` so it is counted and reacted to by the caller.
    pub(super) fn happen(&mut self, kind: HapKind, at: Pos, actor: Option<u32>,
                         target: Option<u32>, foe: Option<String>, weight: u8, text: String,
                         extra: Option<&mut Dwarf>) -> u32;
    pub fn happening(&self, id: u32) -> Option<&Happening>;
    pub(super) fn witnesses(&self, at: Pos, r: i32, except: &[u32]) -> Vec<u32>;
    pub(super) fn tally(&mut self, key: &'static str);   // annals.society += 1
}
// Game gains: #[serde(default)] happenings: Vec<Happening>, next_hap: u32
// Annals gains: #[serde(default)] society: BTreeMap<String, u32>;  Trial gains society too.
pub fn resents(d: &Dwarf, other: u32) -> bool;   // C02 fills; stub returns false

// ---------------- src/game/social.rs ----------------
pub struct Tie { pub other: u32, pub trust: i8, pub respect: i8, pub fear: i8,
                 pub impression: i8, pub feud: u8, pub since: u64 }
pub struct Vow { pub foe: String, pub foe_id: Option<u32>, pub for_whom: u32, pub sworn: u64 }
pub struct Social { pub ties: Vec<Tie>, pub nickname: Option<String>, pub shunned_until: u64,
                    pub vow: Option<Vow>, pub debts: Vec<(u32, u32)>, pub arrived: u64,
                    pub counters: BTreeMap<String, u16> /* duels lost, jailings, outs… */ }
impl Dwarf { pub fn tie(&self, o: u32) -> Tie; pub fn tie_mut(&mut self, o: u32) -> &mut Tie; }
pub fn suspicion(g: &Game, d: &Dwarf) -> i32;    // C03 fills; stub 0
pub fn compat(a: &Dwarf, b: &Dwarf) -> i32;      // C04 fills; stub = today's sociability term
// Dwarf gains: #[serde(default)] pub social: Social.  `relations` keeps regard (decision above).
// Dwarf::called() (dwarf.rs) shows the nickname when there's no epithet: done in C00.

// ---------------- src/game/family.rs (+ saga.rs) ----------------
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kin { Spouse, Parent, Child, Sibling, Grandparent, Grandchild, Cousin, Guardian, Ward }
pub struct Saga { pub kin: Vec<(u32, Kin)>, pub named_for: Option<String>,
                  pub guardian: Option<u32>, pub rite: Option<bool>, pub favourite: Option<u32> }
impl Game { pub fn kin(&self, a: u32, b: u32) -> Option<Kin>;
            pub fn kin_of(&self, a: u32) -> Vec<(u32, Kin)>; }
// Dwarf gains #[serde(default)] pub saga: Saga

// ---------------- src/game/brawls.rs ----------------
pub enum BrawlCause { Grudge, Drunk, Tantrum, Dare, Feud, Guest, Prison }
pub struct Brawl { pub id: u32, pub at: Pos, pub sides: [Vec<u32>; 2], pub cause: BrawlCause,
                   pub started: u64, pub blows: u32 }
impl Game { pub(super) fn start_brawl(&mut self, a: u32, b: u32, c: BrawlCause) -> Option<u32>;
            pub fn in_brawl(&self, d: u32) -> Option<u32>; }   // C03 fills; stub None
// Game gains #[serde(default)] pub brawls: Vec<Brawl>

// ---------------- src/game/errands.rs ----------------
pub enum Errand { Mourn{dead:u32}, Brawl{brawl:u32}, Report{crime:usize}, Steal{item:u32},
                  Feed{prisoner:u32}, Visit{who:u32}, Stocks{until:u64}, Leave, Petition{id:u32},
                  Serve{to:u32,item:u32}, Escort{who:u32}, Dare{until:u64}, Sneak{until:u64},
                  Fetch{item:u32}, Lots, Release{cage:u32} }
// dwarf.rs Task gains: Errand(crate::game::errands::Errand)
impl Game { pub(super) fn do_errand(&mut self, d: &mut Dwarf, e: Errand);   // one match, one arm
            pub(super) fn release_errand(&mut self, d: &mut Dwarf, e: &Errand); } // per package
// Each package adds its arms to do_errand/release_errand (one arm each: trivially mergeable).

// ---------------- src/game/council.rs ----------------
pub enum PetitionKind { Temple{god:u8}, Priesthood{god:u8}, Residency{visitor:u32},
    Citizenship{visitor:u32}, Complaint{against:Option<u32>, about:String},
    Barony{candidate:u32}, Banish{who:u32}, Mercy{prisoner:u32}, Feast }
pub enum PetitionState { Open, Granted(u64), Refused(u64), Broken(u64) }
pub struct Petition { pub id: u32, pub from: u32, pub kind: PetitionKind, pub made: u64,
                      pub due: u64, pub state: PetitionState }
pub struct Council { pub petitions: Vec<Petition>, pub favour: Vec<(u32, i32)>,
    pub bans: Vec<Ban>, pub export_bans: Vec<(ItemKind, u32, u64)>, pub crown_snub: bool,
    pub last_election: u64, pub barony_choices: Vec<u32>, pub policy: Policy }
pub struct Policy { pub executions: bool, pub mob_justice: bool, pub drink_on_alert: bool }
impl Game {
    pub(super) fn petition(&mut self, from: u32, kind: PetitionKind, due: u64) -> u32;
    pub fn answer_petition(&mut self, id: u32, grant: bool);  // calls on_petition (dispatch)
    pub(super) fn favour(&mut self, noble: u32, delta: i32);
}
pub fn overseer_grants(o: &Overseer, k: &PetitionKind, rng: &mut Rng) -> bool; // trait table
// on_petition dispatch: Temple|Priesthood → faith.rs; Residency|Citizenship → tavern.rs;
// Complaint|Banish|Mercy → justice.rs; Barony → council.rs; Feast → festivals.rs. Stubs in C00.

// ---------------- src/game/{bottle,faith,tavern,kitchen,grief}.rs ----------------
pub struct DrinkState { pub bac: u16, pub tolerance: u8, pub last_drink: u64,
                        pub spree_until: u64, pub outs: u8 }      // Dwarf.drink
pub enum Tipsy { Sober, Merry, Rash, Stumbling, Sick, Out }
impl Dwarf { pub fn tipsy(&self) -> Tipsy; }                       // stub Sober
pub fn focus_mod(d: &Dwarf) -> i32;                                // stub 0, read by Dwarf::focus
pub struct Faith { pub gods: Vec<(u8, u8)>, pub cursed: Option<u8>, pub prayed: u64 } // Dwarf.faith
pub struct Pantheon { .. as in C10 .. }                            // Game.pantheon
pub struct TavernBook { pub guests: Vec<Guest>, pub bets: Vec<Bet>, pub renown: i32,
                        pub liveliness: u16 }                      // Game.tavern
pub struct Kitchen { pub brews: Vec<NamedBrew>, pub dishes: Vec<Dish> }  // Game.kitchen
pub struct GriefBook { pub missing: Vec<Missing>, pub pending: Vec<(u32 /*mourner*/, u32 /*dead*/,
                       u64)>, pub dark_places: Vec<(Pos, u32, u64)> }  // Game.grief
// Game gains #[serde(default)] exiles: Vec<Dwarf>

// ---------------- pre-added variants and fields on existing types ----------------
// justice.rs CrimeKind += Theft, Murder, Drunkenness, Desecration, Bribery, Escape
// justice.rs Crime += known(default true), reported(default true), witnesses, at, item, accused, punishment
// game.rs ZoneKind += Arena;  Zone += dedicated: Option<u8>, guest: bool
// buildings.rs BuildingKind += Altar, Fishery, Stocks (names, glyphs, costs; no recipes yet)
// positions.rs Position += Hammerer, TavernKeeper (ALL grows; ov_posts rows gated false)
// festivals.rs FestivalKind += Victory, Wedding;  Festival += couple: Option<(u32,u32)>
// ambitions.rs Sense += Mourning, Lobby, Discord, Devotion (fire 0.0 until filled)
// troubles.rs Lack += Mandate
// dwarf.rs Mood += kind: MoodKind {Fey(default), Secretive, Possessed, Fell, Macabre}
// creature.rs Creature += tipsy: u16, feud_with: Option<u32>
// annals.rs DeathNote += last_words: Option<String>
```

**Hook registry (society.rs, all stubs in C00; each package fills only its own):**

| Hook | Called from (C00 wires it once) | Filled by |
|---|---|---|
| `society_daily()` → `inner_daily, grief_daily, feuds_daily, envy_weekly, saga_daily, council_daily, justice_daily, bottle_daily, tavern_daily, faith_daily, kitchen_daily` | `step`, `tick % DAY == DAY/2 + 11` | C01, C02, C03, C04, C05, C06, C07, C08, C09, C10, C11 |
| `society_tick()` → `brawls_tick, grief_tick (corpse finding), tavern_tick (performances)` | `step`, `tick % 10 == 7` | C03, C02, C09 |
| `society_dwarf(d)` → `inner_tick, bottle_tick, child_tick` | `psyche_tick` | C01, C08, C05 |
| `react(&Happening)` → `inner_react, grief_react, feud_react, crowd_react, justice_react, faith_react, tavern_react, saga_react` | `happen` | C01, C02, C03, C04, C07, C10, C09, C05 |
| `leisure(d) -> bool` chain: `festival_activity, grief_visit, spree, help_someone, tavern_leisure, faith_leisure, seek_company, seek_need_zone, (old idle)` | `idle_activity` → `leisure` | C02, C08, C04, C09, C10 |
| `on_death(d, cause)`, `on_arrival(d)`, `on_new_year()` | `kill`, `migrants`, `new_year` | C02, C05 (inherit), C01 (memories) |
| `ov_society(o)` → `ov_comfort, ov_mandate, ov_petitions, ov_feuds, ov_justice, ov_feasts, ov_saga, ov_tavern, ov_faith, ov_breakdown` | `ov_daily` | all |
| `social_senses(o) -> Vec<(Sense, f64)>`, `social_links(a, &senses) -> Vec<(Sense, f64)>` | `brain()` | C02, C03, C06, C10 |
| `ui_society::{sheet_extra, status_society, council_panel, mood_strip, legends_gods}` | ui.rs (one call each) | all |

## Hot-spot discipline (wave 0 refactors, all in C00 step 1, pure moves)

- **H1. Split psyche.rs (777 lines)** into psyche.rs (`psyche_tick`, tantrum, `go_insane`,
  room assignment), council.rs (`Mandate`, `issue_mandate`, `check_mandate`, `nobles_daily`,
  `appoint_leader`), moods.rs (`start_mood`, `do_mood`, `end_mood`, `maybe_strange_mood`) and
  social.rs (`socialize`). C04, C06 and C12 then never share a file.
- **H2. `kill`'s grief loop → `grief.rs::on_death`.** game.rs `kill` keeps one call; C02 and C05
  (inheritance) both write in grief.rs or family.rs, not game.rs.
- **H3. `idle_activity` → `leisure.rs::leisure`** as an ordered chain of `fn(&mut Game, &mut
  Dwarf) -> bool`. Five packages add a link; each adds one line to the chain array plus their own fn.
- **H4. `Dwarf::think` body → memory.rs `feel`.** dwarf.rs keeps a one-line `think`. C01 then
  tunes stress in its own file.
- **H5. society.rs hook registry** (the table above): `step` gets 3 lines, `ov_daily` 1,
  `brain()` 2, `do_task` 1 arm, `migrants` and `new_year` 1 line each.
- **H6. All new fields and sub-structs pre-added** (Dwarf: `inner`, `social`, `saga`, `drink`,
  `faith`; Game: `happenings`, `next_hap`, `brawls`, `council`, `pantheon`, `tavern`, `kitchen`,
  `grief`, `exiles`; Zone, Crime, Mood, Festival, Creature, DeathNote fields). After C00 no package
  edits the `Dwarf` or `Game` struct definitions.
- **H7. All enum variants pre-added** (listed under Interface contracts), with their `name()` and
  `match` arms filled inertly, so no package adds a variant to a shared enum. A non-exhaustive
  match in a far file would otherwise break another branch's build.
- **H8. ui_society.rs**: the dwarf sheet's family, relations and thoughts code (ui.rs ~1700–1805)
  moves into `sheet_extra`. The status screen gets tab 6 with the tab count as a const (app.rs
  `% 5` → `% STATUS_TABS`). The Nobles screen calls `council_panel`.
- **H9. report.rs society section** and `Trial.society`. Packages only call `tally("key")`.
- **H10. Pre-add `ov_posts` rows** for Hammerer and TavernKeeper (gated `false`), so C07 and C09
  don't edit adjacent lines.

## Work packages and waves

| Pkg | Contents | Sessions | Wave |
|---|---|---|---|
| P0a | C00 steps 1–2 (refactors H1–H5, memory.rs, happenings) | 1.5 | 0 |
| P0b | C00 steps 3–4 (H6–H10, kin, ties, petitions, tally, ui scaffolding) | 1.5 | 0 |
| P1 | C01 steps 1–4 (richer thoughts, emotions, two-speed feelings, stress tuning, mood at a glance) | 2.5 | 1 |
| P2 | C03 steps 1–5 (brawl engine, factions, dials, feud ladder, shunning) | 3 | 1 |
| P3 | C05 steps 1–5 (heredity, named for heroes, weddings, siblings, babies) | 2.5 | 1 |
| P4 | C08 all (bottle; step 6 brawl call behind the `start_brawl` stub) | 3 | 1 |
| P5 | C10 steps 1–5 (gods, temples, petitions, design, faith passed on) | 3 | 1 |
| P6 | C11 all (spoilage, fishery, named brews, signature dishes) | 3 | 1 |
| P7 | C06 steps 1–5 (mandates answered, flavour, favourites and bans, office envy, export bans) | 2.5 | 1 |
| P8 | C01 steps 5–7 (memories relived, hardened, drift) | 2.5 | 2 |
| P9 | C02 all (grief, last words, vows, triggers, resentment, homecoming) | 4 | 2 |
| P10 | C04 all, plus C03 steps 6–8 (nicknames, suspicion, pets) | 4 | 2 |
| P11 | C07 steps 1–5 (witnesses, theft, accusations, graded punishment, bribery) | 3 | 2 |
| P12 | C09 steps 1–6 (keeper, bar brawls, performers, gambling, tall tales, guest feuds) | 3 | 2 |
| P13 | C06 steps 6–11 (barony, lobby sense, elections, spin, petitioners, counsel panel) | 3.5 | 2 |
| P14 | C10 steps 6–10 (cursing, divination, desecration, cults, answered prayers) | 3 | 2 |
| P3b | C05 steps 6–10 (orphans, rivalry, inheritance, rite, rebellious youth) | 2.5 | 2 |
| P15 | C07 steps 6–11 (prison life and unrest, banishment, hammerer, public punishment, witch hunts) | 3 | 3 |
| P12b | C09 steps 7–9 (paying guests, residents, arena) | 2 | 3 |
| P16 | C12 all (tantrums with limits, darker moods) | 2 | 3 |

**Wave 0 (serial, one agent):** P0a then P0b. Nothing else starts until P0b is merged and the
`m3-base` batch is saved.

**Wave 1 (up to 7 agents at once):** P1, P2, P3, P4, P5, P6, P7.
- Disjoint modules: memory.rs / brawls.rs, social.rs, feuds.rs / family.rs, saga.rs /
  bottle.rs / faith.rs / kitchen.rs / council.rs.
- Overlaps to merge by function, all different functions:
  - P2 `drama_daily` vs nothing else in wave 1;
  - P3 `start_festival` (guest filter) vs P4 `festival_drink`;
  - P4 game.rs `travel` and `do_task` DrinkItem vs P6 game.rs craft completion;
  - P5 overseer.rs `ov_fit` vs P6 overseer.rs workshop list vs P7 troubles.rs.
- **Serialize:** P1 should merge **first** in the wave (it changes how stress lands, and the
  others' tests assert thoughts but not stress totals), then the rest in any order. P2 and P4: P4
  step 6 uses `start_brawl`; while P2 is open it calls the stub. Merge P2 before P4 if both are
  ready.

**Wave 2 (up to 8 agents):** P8, P9, P10, P11, P12, P13, P14, P3b.
- Needs wave 1: P8, P9 and P14 need P1; P10, P11 and P12 need P2; P12 needs P4; P13 needs P7;
  P14 needs P5; P3b needs P3.
- Same-function risks:
  - **P9 and P11** both touch combat.rs: P9 `combat_awareness`, P11 `skulker_think`. Different
    functions; fine.
  - **P10 and P3b** both touch drama.rs: P10 adds `dares_daily`, P3b nothing there. Fine.
  - **P10 (festival crowd hook) and P12 (`arm_wrestle` bet)** both edit festivals.rs, in different
    functions.
  - **P9 and P13** both call into `resents()`: P9 owns it, P13 uses the stub until P9 merges.
  - **P11 and P13** both use `Errand::Petition` and `Complaint`. P13 owns making complaints,
    P11 owns the justice dispatch of a granted complaint. Contract: `on_petition` →
    `justice::on_complaint(p)`.
- **Serialize:** P8 after P1 (same file, memory.rs). P8 and P9 both edit memory.rs: P9 only
  `resents` (a separate fn), P8 `inner_daily`. Allowed, but merge P8 first.

**Wave 3 (3 agents):** P15, P12b, P16.
- P15 needs P11, P10 (crowds) and P9 (GriefBook). P12b needs P12 and P11. P16 needs P8, P14
  (`tantrum_mischief` desecration check) and M1 S09.
- P15 and P12b both touch justice paths (guest thefts): P12b only calls `commit_crime`. Fine.

**Critical path:** P0a → P0b → P1 → P2 → P11 → P15, about 14 sessions end to end with full
parallelism (57 sessions of work in total).

## Verification per package

Every package: `cargo test --release` prints `test result: ok`, with the package's tests named in
its item's Steps (each step's test lands in the same commit). It must also show that
`one_year_timing` is within +5% of the wave's base (paste the number in the PR).

- **Batch checks are done by the coordinator once per wave, after merging**, with `--autoplay
  --runs 25 --years 3 --seed 5000 --out batches/m3-wN.json` then `--compare` against the previous
  wave:
  - **Wave 0:** identical survival to the M2 baseline (pure refactor), and the society keys exist.
  - **Wave 1 (the risky one):** survival within ±1 fort a year; tantrums plus melancholy −10% or
    better (P1); starvation not up (P6); `drunk_out` > 0 and alcohol deaths ≤ 0.5% (P4); `brawl`
    1–4 per fort-year (P2); `mandate_met` > 0 (P7); `temple_dedicated` > 0 in ≥ 40% of forts (P5).
  - **Wave 2:** survival within ±1; `vow_kept` > 0 somewhere; `wrong_accusation` 10–30% of
    accusations; `relived` ≥ 5 per fort-year; `performance` > 0 where a tavern exists; ambition
    mixes per style not collapsed toward one ambition by lobbying (compare `Trial.ambitions`).
  - **Wave 3:** the milestone release check above.
- **Packages that need a quick spot batch** (5 forts, 1 year, by the agent, ~2 min) before
  handing off, because they change stress or deaths: P1, P4, P9 (vows), P11, P15 (banishment and
  executions), P16 (fell moods).

## Cross-milestone notes

- **One event and memory foundation.** Happenings (with witnesses), `Feeling`/`Memory` and
  `tally()` are what M2 O05 (director camera, highlight reel, last words), O06 (life story, unit
  list), O09 (named episodes), A03 (fight log, revenge) and M7 (interrogation, curses) all need.
  If M2 is built before M3, **build C00's memory.rs in M2** and let O05 and O06 consume it.
- **One petition, decision and favour store.** `Council` with `Petition` and `overseer_grants()`
  serves C06, C09 and C10 here and B02 (envoys, barony, banish, petitions), P02 (crown) and B04
  (traitorous advisors) later. It is the "opinion of the overseer" store; B02's "opinion of civ"
  should sit next to it (`council.civ_favour`).
- **One brawl engine.** C03's brawls.rs serves drunks, tavern fights, tantrums and prison riots,
  and later A02's "bystanders choose sides" and A04's sparring accidents.
- **One errand task.** `Task::Errand(Errand)` gives every social system a way to walk somewhere
  and do a thing without touching the `Task` enum again. M4 visitors, M7 spies and N02 saboteurs
  should use it too.
- **One kin function.** `kin()` and `kin_of()` serve grief, brawls, inheritance, banishment, the
  family tree (O06) and succession (P02).
- **Refactors worth doing first**, even before M1/M2 finish: H1 (split psyche.rs), H2
  (`on_death`), H3 (the leisure chain) and H5 (the hook registry). They are pure moves that make
  every later milestone's parallel work cheaper.
- **Duplicates to build once:** the list in "Order and why". Keep a table in WISHLIST mapping each
  pair to the item that owns the code.
- **Tunables:** if S10 (OTH-04) lands, every M3 constant (memory threshold 40, spirits decay 8%,
  drip /25, bac step 140, shun at 6, vow odds) goes in its data file from day one.
- **Measurement first:** without `annals.society` and the report section, none of M3's balance
  can be judged by the batch. It is the first commit after the refactors.
