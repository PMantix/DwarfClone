# Implementation plan: Milestone 2

Grounded in the code as of October 2026 (DwarfClone, ~37k lines). File:function citations refer
to the current tree. Nothing here edits the repo; every step is one commit with its test, and
WISHLIST ticks go in the same commit (CLAUDE.md).

## Milestone 2: the overseer as a character (v0.4)

Goal (WISHLIST): watching a fort, you can tell which overseer built it, and its halls look planned
and lived in.

### What the code gives us to build on

- `src/game/overseer.rs` (1,899 lines): `Overseer` (style, rooms, `done: Vec<String>` one-off
  flags, ambitions, nudges, layout, level, anchors, `plan_taken`, `plan_len`, troubles' `fixes`
  and `boosts`, `name`, `traits`, `notes` capped at 60 by `Game::note`). The tick hooks are
  `Game::oversee` (every 150 ticks, `ov_rooms`, `ov_orders`, `ov_posts`, `ov_army`, `ov_daily`
  once a day) and `oversee_alert` (every 10 ticks, `ov_alert`). `ov_daily` is a flat list of
  per-system calls, which is good for parallel work.
- **Everything faces east.** The gatehouse is `base.offset(1..14, 0..1)`; doors at x+1, bridge
  at x+2, lever at (3,1), cage traps x+4..8 (`ov_defences`), stone traps x+10..14 (`pursue`
  Fortify), the inner stair at (9,1) (`layout.rs: level_hub`, `ov_daily` burrow), the shaft at
  (11,1) (`ambitions.rs: ov_shaft`), rooms only where `o0.x - 1 >= b.x + 15`
  (`layout.rs: room_fits`, `blueprint.rs: slot_fits`), the plan starts at `b.offset(15,0,0)`
  (`blueprint.rs: plan_start`) and its length is `(world.w - 3 - start.x).clamp(30, 64)`
  (`plan_slot`). There are 55 hard-coded `base.offset`/`b.x +` sites across overseer.rs,
  ambitions.rs, layout.rs and blueprint.rs. `blueprint.rs: axes(dir)` already knows E/W/N/S, but
  `plan()` hard-codes 'E' for spines and absolute x/y arithmetic for the atrium and hub.
- `ov_setup` scores stair spots on `gate_dry` and `roofed` along +x only.
- `personality.rs`: eight traits, `lean`, `layouts`, `voice` (only for notes that start with
  "Saw", from `troubles.rs: ov_troubles`). `roll_personality` uses the game rng.
- `ambitions.rs: brain` has 13 senses (`Sense::ALL`, drawn in `ui.rs: draw_brain`).
- Work orders (`orders.rs`): only "keep N" (`WorkOrder { recipe, keep }`); a missing input
  gives `find_craft` a daily `warn` and a `trouble`; the overseer answers troubles with
  `boosts` (`troubles.rs: fix`). Nothing chains inputs.
- Records: `annals.rs` (DeathNote, SiegeNote), `chronicle.rs` (YearLog, titles), legends via
  `trade.rs: legend` into `overworld.events: Vec<Event { year, text }>`, `milestones`,
  `spotlight.rs` (title/text/at/color, not saved), `LogEntry { text, color, tick }`.
  `Dwarf.thoughts` is capped at 8: **there is no per-dwarf memory of a life**.
- No Cabinet/Chest/Statue building or item kinds; no names for rooms, zones or places; one
  zone per tile (`Tile.zone`); levers link instantly and eat a mechanism (`traps.rs:
  link_lever`); dig time is `mat.dig_ticks()` with skill (`game.rs: Task::Dig`, `work_time`);
  only one gem (Amethyst); no falling, no burial under rubble (`disasters.rs: cave_in` wounds
  for 7); ranged shots only on one z (`military.rs: clear_shot`), and fortifications don't block
  them (`TileKind::solid` excludes Fortification).
- UI: `ui.rs` 5,131 lines; the status bar shows `AUTO: Style · Ambition`; the announcements split
  55/45 with the overseer's notes while it plays; F3 overlay (`draw_perf`); spotlight cards
  (`draw_spotlight`, auto-dismiss after 100 frames when watching, `app.rs: update`); `Mode`
  enum in app.rs; `p` is taken (stockpiles), free keys in `key_normal` include `P E F J N O R T
  U V W Y s t x e f g h a`.

**Order and why**

1. **Wave 0 (foundations, 2 sessions, two agents in parallel):** the facing refactor (O02 step 1,
   pure refactor of the 55 east-only sites) must land before anyone else edits overseer.rs,
   layout.rs or blueprint.rs; and a scaffolding commit adds the shared stores M2 needs (saga,
   places/names, LogEntry kinds and positions, glyphs module, Trial stats, Mode placeholders,
   Game fields) so later branches never touch the `Game` struct, `step`, `Mode` or `Trial` again.
2. **O01 then O02** (the overseer's face and its halls): O01 is small and touches personality.rs
   and the autoplay screen; O02 needs the refactor and is the critical path for O03's furniture
   slots and O04's gatehouse positions.
3. **O03 and O09 share places/names**; O03's "names" package and O09's episodes can run in
   parallel once the places store exists. O03's furniture waits for O02's blueprint transform.
4. **O04** needs M1's S04 (whole gate, bridge on a lever) and S07 (archers) to be merged, the
   gatehouse.rs move from wave 0, and O02's frame. It is the longest single feature chain
   (pit-bridge needs falling).
5. **O05, O06** are mostly UI and run in parallel with the game-side items; they need the saga
   and LogEntry foundations; O06's death scenes use O05's last words if present (soft).
6. **O07** needs the saga (journal) and, for spin, SiegeNote; independent of O02-O04.
7. **O08, O10, O11** are self-contained systems (orders, digging, building) and can go any time
   after wave 0; O08's vein mining (ECO-14) is built once, in O10 (DIG-11).
8. **O12** last: it joins O02 (grand works on the plan), O03 (names), O07 (faith) and the moods.

Critical path: wave 0 → O02 (frame, transform, facing choice) → O04 (gatehouse stages) → M2
release batch. Everything else hangs off wave 0 in parallel.

Dependencies on other milestones: O04 needs S04 and S07 (M1). S10 (tunables file) should land
before M2's balance work so the thresholds below (faith, tiers, patience) live in the knobs
file. M3's C01 (inner life) and C06 (council) and M8's P02 (crown and succession) build on O06's
life story, O07's faith and succession; M5's A03 (legends of the blade) builds on CRF-02 named
weapons from O03. M8's P01 (fallen forts live on) reads O07's journal.

**Release check**

- `cargo test --release` green, with every test named below.
- The determinism check after wave 0: the batch `--autoplay --runs 25 --years 3 --seed 5000`
  run before and after the facing refactor gives byte-identical `--out` JSON (the refactor is
  pure). Later steps that change the rng stream say so and re-baseline.
- The M2 batch (`batches/m2.json` vs the M1 release batch): survival per year not worse by more
  than 2 forts per year; the new "Overseers" section of the report (see Trial `M2Stats` below)
  shows: plan fill ≥ 60% of slots on level 0 for planned layouts by year 2; ≥ 80% of forts alive
  at year 3 with a named hall and smoothed main halls; every siege named; every fort a highlight
  reel with ≥ 1 clip per siege; ≥ 1 succession in ≥ 20% of forts; Delver forts mine ≥ 1.5× the
  ore of Builder forts; no "needs …" announcement repeated for the same shop and input within a
  season.
- The "same seed, other traits" check (O01's test, plus `--traits` in the batch): two runs of 10
  forts with the same seeds, `--traits cautious,greedy` vs `--traits vain,cheerful`, differ in
  layout choice, trap count, ambition mix and grand works in ≥ 8 of 10 forts.
- A thing to watch: open *Watch a fort*, seed 5005, Warlord: within three years you should see
  the gatehouse grow (trap hall, pit bridge, towers), a named siege, the overseer die or be
  deposed at least sometimes, and the reel play back from the game-over page.

---

### Wave 0: shared foundations (not WISHLIST items; 2 sessions)

These are the hot-spot refactors and interface contracts every M2 item builds on. Two agents,
disjoint files.

#### W0-A Facing refactor (= O02 step 1) — 1 session

- **Design:** `src/game/layout.rs` gains

  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
  pub enum Facing { #[default] East, West, North, South }
  impl Facing {
      pub const ALL: [Facing; 4];
      /// (forward, side) unit vectors: East = ((1,0),(0,1)), West = ((-1,0),(0,-1)),
      /// North = ((0,-1),(1,0)), South = ((0,1),(-1,0)) — a rotation, so mirrored
      /// layouts stay mirrored.
      pub fn axes(self) -> ((i32, i32), (i32, i32));
      /// A heading in the plan's own frame ('E' forward, 'N' left, 'S' right, 'W' back)
      /// turned into a map heading.
      pub fn turn(self, local: char) -> char;
      pub fn name(self) -> &'static str; // "east", ...
  }
  ```

  and `Overseer` (overseer.rs) gains `#[serde(default)] pub facing: Facing` plus

  ```rust
  impl Overseer {
      /// `base` moved `fwd` along the halls' heading and `side` across it.
      pub fn rel(&self, fwd: i32, side: i32, dz: i32) -> Pos;
      /// The same from the stair top (the yard).
      pub fn rel_top(&self, fwd: i32, side: i32) -> Pos;
      /// Inverse: (fwd, side, dz) of `p` relative to base.
      pub fn local(&self, p: Pos) -> (i32, i32, i32);
  }
  ```

  Every east-only site is rewritten through these with East giving identical numbers:
  `overseer.rs: ov_setup` (gate dig, `gate_dry`, `roofed`), `ov_alert` (gate doors incl.
  `top.offset(-3,0,0)`, squad station `base.offset(4+s,0,0)`), `gate_lever`, `gate_bridges`
  (compare `o.local(b.pos)`), `ov_defences`, `ov_daily` (burrow box and inner stair), `ov_mine`
  (scan box in local coordinates), `ambitions.rs: pursue` (stone traps, yard), `ov_shaft`,
  `layout.rs: level_hub, room_fits (fwd >= 15), room_spots, tunnel (never fwd < 14 on the main
  level)`, `blueprint.rs: plan_start, slot_fits`. Tests that read `base.offset` (overseer.rs:
  `the_gate_is_whole_and_mended`, `the_shelter_reaches_every_room`) switch to `o.rel`.
- **Test:** `layout.rs: facing_rel_round_trips` (for every facing, `local(rel(f,s,dz)) ==
  (f,s,dz)`, and East equals `base.offset(f,s,dz)`); the full suite green; the coordinator runs the
  25-fort batch and `--compare` must show 0 forts changed.
- **Hot spots:** overseer.rs, ambitions.rs, layout.rs, blueprint.rs: many lines, which is why it
  goes alone and first. No `Game` fields.

#### W0-B Scaffolding: stores, hooks, placeholders — 1 session

One commit that adds empty-but-working foundations and **all** new `Game` fields, `Mode`
variants and `step` hooks M2 needs, so packages fill in new modules without touching the shared
structs again. Each field is `#[serde(default)]`.

1. **`src/game/saga.rs`** (new): the fortress's memory of what happened to whom.

   ```rust
   #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
   pub enum SagaKind { Arrived, Born, Married, Died, Wounded, Mood, Artifact, Epithet,
       Appointed, Soldier, Expedition, Siege, SiegeEnd, Monument, Feat, Madness, Recovered,
       Overseer, Named, Inspired, Other }
   #[derive(Clone, Debug, Serialize, Deserialize)]
   pub struct SagaEvent { pub id: u32, pub tick: u64, pub kind: SagaKind,
       pub who: Vec<u32>, pub at: Option<Pos>, pub text: String, pub weight: u8 }
   #[derive(Clone, Debug, Default, Serialize, Deserialize)]
   pub struct Saga { pub events: Vec<SagaEvent>, next: u32 }
   impl Game {
       /// Remember something (weight 1 small .. 9 legendary). Keeps at most 6,000; when
       /// full, drops the oldest weight-1 and -2 events first.
       pub(super) fn record(&mut self, kind: SagaKind, who: &[u32], at: Option<Pos>,
                            text: String, weight: u8) -> u32;
       pub fn saga_of(&self, dwarf: u32) -> impl Iterator<Item = &SagaEvent>;
       pub fn saga_since(&self, tick: u64) -> impl Iterator<Item = &SagaEvent>;
   }
   ```

   `Game.saga: saga::Saga`. W0-B also adds the first writers, one line each, in the places that
   already tell the story: `game.rs: kill` (Died, w 3; 6 for heroes), `family.rs` births and
   marriages, `heroes.rs: earn_epithet`, `psyche.rs: do_mood` artifact, `psyche.rs: go_insane`,
   `positions.rs: appoint`, migrants' arrival, `annals.rs: note_siege` and the siege's end in
   `annals_tick`, `megaprojects.rs: dedicate_monument`, `expeditions.rs` set out/return.
2. **`src/game/names.rs`** (new, CRF-08): original word lists and

   ```rust
   pub enum NameTheme { Hall, Battle, Weapon, Place, Work, Automaton }
   /// A name from a seed (a hash, so naming never draws on the game's rng and never
   /// changes a batch).
   pub fn coin(seed: u64, theme: NameTheme) -> String;
   pub fn hash(parts: &[u64]) -> u64;
   ```
3. **`src/game/places.rs`** (new, WLD-02/ENG-16 foundation):

   ```rust
   #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
   pub enum PlaceKind { Hall, Gate, Battlefield, Tomb, Deep, Road, Work }
   #[derive(Clone, Debug, Serialize, Deserialize)]
   pub struct Place { pub id: u32, pub name: String, pub kind: PlaceKind, pub a: Pos,
       pub b: Pos, pub why: String, pub since: u64, pub weight: u8, pub by_player: bool }
   impl Game {
       pub fn place_at(&self, p: Pos) -> Option<&Place>;  // smallest box containing p
       /// Name (or rename, if `weight` beats the old name's and the player didn't
       /// name it) the place covering a..b; returns its id.
       pub(super) fn name_place(&mut self, kind: PlaceKind, a: Pos, b: Pos, name: String,
                                why: String, weight: u8) -> u32;
   }
   ```

   `Game.places: Vec<places::Place>`. A look-panel line (`ui.rs: look_lines`, one `if let`)
   "In the Hall of the Unbeaten (named for …)".
4. **LogEntry gains a place and a kind** (UI-06/07 foundation): `game.rs: LogEntry { text,
   color, tick, #[serde(default)] at: Option<Pos>, #[serde(default)] kind: LogKind }` with
   `LogKind { Other, Death, Combat, Siege, Mood, Birth, Arrival, Building, Job, Trade, Weather,
   Overseer, Legend }` and `LogKind::classify(text, color)` (moved from `ui.rs: report_kind`).
   `Game::msg` stays (classifies); new `Game::msg_at(text, color, at, kind)`.
5. **`src/glyphs.rs`** (new): `tile_look`, `foliage`, `water_glyph` moved out of ui.rs, returning
   `(char, (u8,u8,u8) fg, (u8,u8,u8) bg)`; `ui.rs` wraps them in `rgb()`. Needed by O05 (reel,
   export) without depending on ratatui in game code. Plus `pub fn unit_glyph(g, p)` for units,
   items and buildings on a tile (lifted from `draw_map`).
6. **Trial stats:** `overseer.rs: Trial` gains `#[serde(default)] pub m2: M2Stats` (new struct in
   new `src/game/m2stats.rs`: facing, levels, plan_fill (taken, slots), smoothed_pct,
   named_places, traps, gate_stage, reigns, faith_end, clips, ore_mined, chained_orders,
   inspirations, all `u32`/`f32` with defaults) filled by one call `g.m2_stats()` at the end of
   `run_trial`; `report.rs` gets one function `overseers(trials)` printing them by style and by
   trait, and a line in `compare`.
7. **Mode placeholders** in `app.rs: Mode`: `OverseerTraits { sel }`, `Journal { year }`,
   `Family { id, sel }`, `Reel { clip, frame }`, `Announce { sel }` — each drawing "coming soon"
   and Esc back, with their key handlers in **new files** (`src/app_m2.rs` with `impl App { fn
   key_journal(..), .. }`; App's fields are pub) and their draw functions in **new files**
   (`src/ui_overseer.rs`, `src/ui_units.rs`, `src/ui_reel.rs`), so ui.rs and app.rs get one
   match arm each.
8. **Step hooks:** one `self.m2_daily()` call in `Game::step` at `tick % DAY == DAY/2 + 9` and
   one `self.m2_tick()` at `tick % 20 == 11`, both defined in `src/game/m2hooks.rs` as a list of
   calls into the M2 modules (initially empty). Packages add one line to m2hooks.rs, never to
   `step`.
9. **Overseer hooks:** `ov_daily` gains one call `self.ov_m2(o)` (new `src/game/ov_m2.rs`, a list
   of per-item calls). `Overseer` gains, all `#[serde(default)]`: `levels: Vec<LevelPlan>`,
   `dwarf: Option<u32>`, `faith: i32` (default via `fn sixty()`), `faith_log: Vec<(u64, i32,
   String)>`, `reigns: Vec<Reign>`, `journal: Vec<JournalPage>`, `gate_stage: u8`,
   `orders_seen: u32`, `seen_deaths: usize`, `seen_sieges: usize`, `commands: Vec<(Command,
   u64)>`, `tier: u8`, `adopted: Option<u32>` (types defined in the modules that own them, as
   empty structs/enums to start).
10. **Gatehouse move:** `ov_defences`, `gate_lever`, `gate_bridges` and the bridge half of
    `ov_alert` move verbatim into new `src/game/gatehouse.rs` (`impl Game`), so O04 and M1
    follow-ups edit that file, not overseer.rs.

- **Test:** `saga.rs: the_saga_keeps_the_big_things` (record 7,000 events of mixed weight: ≤ 6,000
  kept, all weight ≥ 5 kept, `saga_of` finds a dwarf's), `names.rs: names_are_stable_and_varied`
  (same seed same name; 200 seeds ≥ 150 distinct), `places.rs: a_heavier_deed_renames_a_place`,
  `game.rs: old_saves_load_without_m2_fields` (serialize a Game, strip the new keys from the
  JSON, deserialize), and `ui.rs: render_snapshot` still passing after the glyphs move.
- **Hot spots:** `Game` struct (+3 fields: saga, places; LogEntry changes), `step` (+2 lines),
  `Mode` (+5 variants), `Trial` (+1 field), `Overseer` (+12 fields), report.rs (+1 fn), ui.rs
  (glyph functions moved, +5 arms), app.rs (+5 arms). Done once, by one agent.

---

### O01 Overseer personalities — remaining steps  (effort: 1.5 sessions; risk: low)

Done so far (7579248): name, two traits, temper sense, layouts, gate patience, army share,
voice on "Saw" notes, `A`/`r` reroll. Left: pick the traits by hand, more notes in its voice,
OVR-24 watch and nudge, and the Done-when test.

- **Design:**
  - *Pick by hand:* `Mode::OverseerTraits { sel }` (placeholder from W0-B), opened with `t` on the
    Autoplay screen. Eight rows with ☐/☑ and the blurb; Space toggles; at most two; a clash
    (`Trait::clashes`, made `pub`) refuses with a flash; Enter calls
    `Game::set_traits(&mut self, traits: Vec<Trait>) -> Result<String, String>` (new, in
    personality.rs), which also re-picks the layout only if the overseer hasn't dug in
    (`o.base.is_none()`). `n` on the same screen renames (a one-line text field; the world-setup
    seed field in `app.rs: key_world_setup` is the pattern). CLI: `--traits cautious,vain` passed
    through `main.rs` to a new `run_trial_with(seed, style, years, traits: Option<Vec<Trait>>)`
    (`run_trial` calls it with None); the title screen's *Watch a fort* gets the same picker.
  - *More of its voice:* replace `Overseer::voiced(&self, text)` with
    `voiced(&self, kind: NoteKind, text: &str) -> String`, `NoteKind { Saw, Dug, Fitted, Gate,
    Ambition, Siege, Death, Trade, Venture, Command }`, and per-trait phrase tables
    `Trait::voice_for(kind) -> &'static [&'static str]` (3-4 openers each, original). The opener
    is chosen with `names::hash(&[tick, notes.len()])`, never the game rng, so batches don't
    shift. `Game::note_ov` gets a `kind` argument; every `Self::note(o, …)` call in overseer.rs,
    ambitions.rs, troubles.rs, layout.rs is tagged (mechanical). New notes where it has
    something to say: `ov_m2 → ov_reflect(o)` reads `annals.deaths[o.seen_deaths..]` and
    `annals.sieges[o.seen_sieges..]` daily: sentimental mourns by name ("With a heavy heart, it
    had Kib buried in the east tomb"), bold one sneers at a withdrawn siege, cautious one frets,
    greedy one counts the trade.
  - *Watch and nudge (OVR-24):* `Overseer::intent(&self, g: &Game) -> String`: the next thing it
    will do, from `o.wants.first()`, the next plan room (`plan(o.style)[o.planned]`), the gate's
    next part (`gatehouse.rs` reports what's missing), or the ambition: "Next: a tavern
    (Revel)", "Next: the drawbridge, once 2 mechanisms are made". Shown after the ambition in the
    status bar (`ui.rs: draw`, the AUTO span) when width allows, and at the top of the brain's
    side panel. Spotlight cards get actions: `spotlight.rs: Spotlight` gains `pub actions:
    Vec<Command>` (default empty) and `Game::spotlight_with(title, text, at, color, actions)`;
    `Command { RaiseBridge, Shelter, StandDown, Muster, Recall }` with `key()`/`label()`. The
    scout warning (S03), siege arrival, a beast near the halls and a losing expedition attach
    commands. While the card is up, its keys call `Game::command_overseer(c: Command) ->
    String`: stored in `o.commands` with an expiry; `gatehouse.rs` and `ov_alert` read them
    (RaiseBridge forces `want` for DAY/2; Shelter forces the civilian alert; Muster calls
    `force_ambition(Muster)`; Recall brings an expedition home if S06's early return exists).
    A stubborn overseer refuses one time in three ("Grudgingly, it would not be told: the
    bridge stays down"), a cautious one obeys a Shelter at once. Card footer: "b raise the
    bridge · s shelter · Enter go on".
- **Steps:**
  1. *Traits by hand* · personality.rs (`set_traits`, `clashes` pub), app_m2.rs
     (`key_overseer_traits`), ui_overseer.rs (`draw_overseer_traits`), app.rs Autoplay `t`,
     main.rs `--traits`, overseer.rs `run_trial_with` · test `personality.rs:
     traits_can_be_chosen_but_not_clashing` (Cautious+Bold refused, Vain+Pious accepted and
     shown in `title()`), `ui.rs: pick_the_overseers_traits_by_hand` (App test: `A`, `t`, two
     Spaces, Enter → `o.traits` as chosen).
  2. *Its voice in every note* · personality.rs (`NoteKind`, `voice_for`, `voiced`),
     all `note_ov` call sites, ov_m2.rs `ov_reflect` · test `personality.rs:
     every_trait_speaks_in_its_own_voice` (for each trait, `voiced(Dug, "Dug out a tavern.")`
     differs from the plain text and from another trait's; same tick → same text) and
     `overseer.rs: a_sentimental_overseer_mourns_by_name` (kill a dwarf with
     `kill_for_test`, step a day: a note names them).
  3. *Its intent, and commands from the card* · overseer.rs (`intent`), gatehouse.rs (read
     `o.commands`), spotlight.rs (`actions`, `spotlight_with`), app.rs (keys on a card), ui.rs
     `draw_spotlight` footer and status bar · tests `overseer.rs: the_overseer_says_what_it_will_do_next`
     (fresh fort after 300 ticks: intent non-empty and names a room or the gate) and
     `gatehouse.rs: raise_the_bridge_from_the_card` (fort with a linked bridge, no army,
     `command_overseer(RaiseBridge)`; within 200 ticks the bridge is raised; a stubborn one may
     refuse but then says so in a note).
  4. *Done when* · test `personality.rs: two_tempers_build_different_forts`: `Game::new(5)`,
     Steady, traits `[Cautious, Greedy]` vs `[Vain, Cheerful]` (set before the first oversee),
     two seasons each; fingerprint = (layout, ambitions taken, Tavern/Temple/Grand rooms, trap
     buildings, raised_at patience). Assert fingerprints differ, the cautious fort has at least as
     many traps, the vain one has the showier layout or a Grand room or more Architect odds.
- **Overseer & screen:** this item *is* the overseer's face: the trait picker, its voice in the
  notes pane (bottom right while it plays), the intent in the top bar and brain, and commands on
  the spotlight cards. Batch: the report's "Overseers" section groups forts by trait pair.
- **Measure:** `--traits` A/B batches as in the release check (≥ 8/10 forts differ); survival
  must not move (voice and intent use no rng).
- **Risks / open questions:** the status bar is already crowded at 130 columns; the intent is
  dropped below 150 columns (show it in the notes pane title instead). Should commands also
  exist when the player plays (they're just the lever and the alert)? Proposal: only when the
  overseer plays.
- **Parallel work:**
  - **Writes:** personality.rs (`set_traits`, `NoteKind`, `voice_for`, `voiced`), ov_m2.rs
    (`ov_reflect`), overseer.rs (`intent`, `run_trial_with`, note-call tags), ambitions.rs and
    troubles.rs (note-call tags only), gatehouse.rs (read commands), spotlight.rs, app_m2.rs,
    ui_overseer.rs, main.rs (`--traits`).
  - **Reads / depends on:** W0-A (gatehouse positions), W0-B (Mode placeholder, `ov_m2`,
    `o.commands`, `names::hash`).
  - **Hot spots:** overseer.rs: one new fn + tags on note calls (one-word edits on many lines:
    land step 2 early in wave 1 or other overseer.rs branches will conflict on the same lines);
    ui.rs: two small edits (status bar span, card footer); app.rs: one arm for card keys.
  - **Agent size:** one agent, 1.5 sessions.

### O02 Plans that fit the land  (effort: 3.5 sessions after W0-A; risk: medium)

- **Design:**
  - *Plans in their own frame.* `blueprint.rs: plan(layout, start, skip, len)` becomes
    `plan(spec: &LevelPlan, start: Pos) -> Blueprint`: it draws in the canonical frame (origin
    (0,0,z), spine heading 'E', cross halls 'N'/'S' as today), then `Blueprint::transform(&mut
    self, f: impl Fn(Pos) -> Pos)` maps every seg tile, slot tile, door, bed (and O03's
    furniture spots) through `start + rotate(facing)`, and recomputes each `Slot.a` (min x/y of
    its tiles) and `Slot.shape` (w/h swapped for North/South; Pillared/Round keep explicit
    tiles, `Room.tiles` is what's dug). The atrium (`Layout::Radial`) and hub (`Spokes`) use
    local coordinates too.
  - *Level specs.* New in blueprint.rs:

    ```rust
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub enum LevelRole { #[default] Main, Living, Workshops, Stores, Grand }
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct LevelPlan { pub level: i32, pub layout: Layout, pub facing: Facing,
        pub len: i32, pub cross: i32, pub role: LevelRole }
    ```

    `Overseer.levels: Vec<LevelPlan>` (W0-B). Migration: if `levels` is empty and `plan_len >
    0`, level 0 is `LevelPlan { level: 0, layout: o.layout, facing: East, len: plan_len, cross:
    20, role: Main }` so old saves keep their plan. `plan_len` stays for old saves only.
  - *Choosing the side.* `ov_setup` picks the layout first (moved before the stair search; this
    changes the rng order once, so re-baseline), then for each candidate stair spot and each
    `Facing` it applies the existing `gate_dry`/`roofed` checks along that facing (via a
    temporary `Overseer` with that facing and base, using `rel`). Of the 6 best spots, it scores
    each facing by `plan_fit(&LevelPlan, start) -> u32` (new, blueprint.rs): the count of the
    first 16 slots for which `slot_fits` holds (≈ 16 slots × 60 tiles × 9 = 9k tile reads ×
    24 combinations, once per fort). Best (fit, stair score) wins. Note: "It faces its halls
    west, where the rock runs deepest."
  - *Stretching to the space.* `fn run_space(&self, o, start, facing) -> (i32 len, i32 cross)`:
    walks forward from `start` in a 2-wide band until 3 consecutive tiles fail `tunnel_ok` or the
    map edge (len clamped 30..=96), and sideways at the band's middle for cross (clamped 12..=26).
    The blueprint uses `cross` for the lengths now hard-coded as 20 (Orderly), 18 (Symmetric), 22
    (Grid, Spokes). Computed once per level when the level is opened; stored in the LevelPlan.
  - *Levels with a trade.* `layout.rs: plan_room` asks `fn level_for(&mut self, o, kind) ->
    Option<i32>`: (1) a level whose role takes `kind` (`LevelRole::takes(kind)`: Living →
    Bedrooms/Dormitory/Hospital/Tavern; Workshops → Workshops/Stockpile; Stores → Stockpile/Tomb;
    Grand → Grand; Main → anything) and still has a free fitting slot; (2) else level 0 if it
    has room and kind isn't Bedrooms while a Living level exists; (3) else open the next level:
    `role = if kind is Bedrooms/Dormitory { Living } else if Workshops/Stockpile { Workshops }
    else if Grand { Grand } else { Main }`, with its own layout from `pick_layout_for(o, role)`
    (Living leans Grid/Orderly, Workshops leans Grid/Spokes, plus the traits' lean), facing
    re-chosen by `plan_fit` from the inner stair, len/cross from `run_space`. The existing
    caverns, aquifer and `z < 3` guards stay. The piece lists in `plan()` take the role:
    `pieces(role, default)` swaps to Cells-heavy rows for Living and Large-heavy for Workshops.
  - *The ghost plan.* `Game::plan_ghost(&self, z: i32) -> Vec<(Pos, Ghost)>` (blueprint.rs):
    redraws the level's blueprint (pure) and returns undug tiles as `Ghost::{Hall, Free(Class),
    Taken(RoomUse), Door}`. `App.ghost: Option<GhostCache { key: (u64 dig_version, usize
    plan_taken.len(), i32 z), cells: HashMap<Pos, Ghost> }>`; `P` toggles (WISHLIST says `p`,
    which is stockpiles: we use `P`). `draw_map` paints ghost cells only on solid undug tiles:
    halls `░` dim blue, free rooms `·` with the class letter at the centre (c s m L), taken rooms
    the RoomUse letter in amber, doors `+`. A legend line in the sidebar while it's on.
- **Steps:**
  1. *(W0-A, above.)*
  2. *Plans drawn in their own frame* · blueprint.rs (`plan` signature, `transform`, local atrium
     and hub), layout.rs callers · test `blueprint.rs: plans_turn_cleanly`: for every facing and
     planned layout, the invariants of `plans_leave_rock_between_rooms_and_halls` hold, the slot
     count equals East's, and for East the slots are identical to before (compare against the old
     `plan()` kept under `#[cfg(test)]` for one commit, or a stored hash).
  3. *The overseer faces where the rock is* · overseer.rs `ov_setup`, blueprint.rs `plan_fit`,
     LevelPlan migration · test `blueprint.rs: a_fort_with_rock_only_to_the_west_still_fills_its_plan`
     (the Done-when): `Game::new(5)`, then for every z below the surface and x > embark.x + 4 set
     tiles to `Open` + revealed (a cut-away world; a helper `carve_away(g, |p| p.x > x0)` in the
     test); hand to a Steady overseer with `layout = Orderly`, nudge Expand +3, run 2 seasons;
     assert `o.facing == West`, ≥ 6 rooms from the plan (`plan_taken` grows, rooms reach stage
     1), and every room's tiles lie west of the stair.
  4. *Plans stretch to the space* · layout.rs/blueprint.rs (`run_space`, `cross` used in
     `plan`), LevelPlan creation · tests `blueprint.rs: the_plan_stretches_to_the_rock` (a 200-wide
     map: len > 64; a map with rock cut 40 tiles out: len ≤ 40 and every slot fits) and the
     existing layout tests.
  5. *Deeper levels with their own trade* · layout.rs (`level_for`, `pick_layout_for`),
     blueprint.rs (`LevelRole`, `pieces`) · test `layout.rs: deeper_levels_have_their_own_trade`:
     a dug-in Builder (300 ticks), call `plan_room` for Dining, Workshops, Stockpile ×2, Tavern,
     Bedrooms ×8 (marking rooms as `o.rooms`): every Bedrooms room lands on a level whose role is
     Living once level 0 is full, and that level's layout was picked separately (`o.levels.len()
     ≥ 2`).
  6. *`P` shows the plan as a ghost* · blueprint.rs (`plan_ghost`), app.rs (`P`, cache field),
     ui.rs `draw_map` (one block), help text · test `ui.rs: the_plan_shows_as_a_ghost` (App with
     an overseer after 600 ticks, press `P`, dump 130×40: a `░` and the legend appear; press `P`
     again: gone) and `blueprint.rs: the_ghost_marks_free_and_taken_rooms`.
- **Overseer & screen:** the overseer is the only user of plans; notes when it chooses a facing
  and opens a level ("Opened a level for bedrooms at depth 6, laid out as a grid of lanes,
  facing south"); the Autoplay screen lists levels (role, layout, facing, rooms taken/total);
  `P` overlay; the brain's side panel shows the facing. Chronicle: "The halls were carried down
  to a third level" when a level opens (YearLog gains nothing; it's a saga Overseer event).
- **Measure:** `M2Stats.plan_fill` and `levels`; batch target: planned-layout forts reach ≥ 60%
  of level-0 slots by year 2 (today rooms often stop at a dozen on narrow sites); notes "Gave up
  on a … nobody could reach" fall by half; survival unchanged within noise.
- **Risks / open questions:** the gatehouse turning changes where sieges walk; the S04 replay test
  must still pass for East. Rotations interact with the map edge (`slot_fits` keeps 2 tiles
  margin; North/South maps are shorter on small embarks). A Living level below cavern roofs is
  still refused for non-Delvers (good). The ghost overlay is per z and redrawn only on cache
  misses. Decision for the user: `P` instead of `p` for the overlay.
- **Parallel work:**
  - **Writes:** blueprint.rs (`plan`, `transform`, `plan_fit`, `LevelPlan`, `LevelRole`,
    `pieces`, `plan_ghost`), layout.rs (`plan_room`, `level_for`, `pick_layout_for`,
    `run_space`), overseer.rs (`ov_setup` only), personality.rs (`pick_layout_for` lean), app.rs
    (`P`, one field), ui.rs (`draw_map` ghost block).
  - **Reads / depends on:** W0-A `Facing`, `Overseer::rel/local`; W0-B `Overseer.levels`.
  - **Hot spots:** overseer.rs `ov_setup` (one function); ui.rs `draw_map` (one inserted block
    guarded by `app.ghost`); app.rs one key + one field (already added in W0-B if listed there).
  - **Agent size:** one agent, 2 packages (steps 2-4, steps 5-6), sequential.

### O03 Halls that look lived in  (effort: 5 sessions; risk: medium)

- **Design:**
  - *Wealth tiers.* `Overseer.tier: u8` recomputed daily in `ov_m2 → ov_tier`: 0 below
    `wealth/pop` 1,500; 1 (smooth) at 1,500 or Builder from year 1/2; 2 (engrave) at 3,000 or vain;
    3 (engrave bedrooms and noble suites, gem engravings) at 6,000. Reckless caps at 1.
  - *Finishing the halls (step 1).* `ambitions.rs: ov_detail` becomes `detail.rs: ov_finish(o)`
    called from `ov_m2` for every style from tier 1 (the ambitions keep calling it with
    `engrave = true` as now): smooth the main hall segments first (the plan's spine and the
    atrium/hub: `blueprint.rs: plan_halls(o, level) -> Vec<Pos>`), then the gathering rooms, then
    (tier 2) engrave them; (tier 3) bedrooms of nobles and the suite. Progress kept as
    `o.done` keys as today. `ov_labors` (overseer.rs) asks for `Labor::Engraving` 1 + pop/20 from
    tier 1 (today only Builders).
  - *Furniture (step 2).* New `BuildingKind::{Cabinet, Chest, Statue}` and
    `ItemKind::{Cabinet, Chest, Statue, Figurine}`; recipes `WoodCabinet`, `WoodChest`
    (Carpenter), `StoneCabinet`, `StoneChest`, `Statue` (Mason), `Figurine` (Craftsdwarf, stone
    or bone); `psyche.rs: building_value` values them; build-menu letters. Bedroom cells in
    `blueprint.rs: side` get `furn: Vec<Pos>` (the two other back-wall tiles; Slot →
    `Room.furn`, `#[serde(default)]`). `ov_fit` Bedrooms places beds as now; `ov_m2 →
    ov_furnish` (daily, tier ≥ 1) places Cabinet then Chest in finished cells as stock allows,
    and `ov_orders` keeps 2+pop/6 of each from tier 1.
  - *Noble suites.* `RoomUse::Suite` (new variant; old saves never have it). When a dwarf with a
    `Noble` lacks `room_demand()` (psyche.rs `nobles_daily` already thinks about it), `ov_m2 →
    ov_suites` pushes `wants: Suite`; `blueprint::wants(Suite) = [Large]`. `ov_fit` Suite splits
    the slot into thirds along its length: a bedroom (bed, cabinet, chest; Bedroom zone owned by
    the noble via `set_zone_owner`), an office (table, chair; Office zone, owned), a dining room
    (table, 2 chairs; Dining zone). Doors between thirds. Owner reassigned if the noble dies.
  - *Statues and the monument at the heart (step 3).* Blueprint `features: Vec<(Pos, Feature)>`
    with `Feature::{Statue, Monument}`: the atrium (Radial) gets the Monument spot at its centre
    and Statue spots by the inner pillars; the hub (Spokes) a Monument at its centre; Orderly's
    great hall a Statue every 6 tiles along its sides at tier 2. `ov_m2 → ov_adorn` places
    statues on free feature spots from tier 1. `overseer.rs: ov_daily` and `ambitions.rs: pursue`
    Prosper choose the monument spot by `fn monument_spot(&self, o) -> Option<Pos>` (the plan's
    Monument feature if any, else today's `e.offset(0,6,0)` search).
  - *Statues of things (CRF-21).* A Statue or Figurine item's `label` names a subject chosen when
    made (`game.rs` craft completion, one call to `fn subject(&mut self) -> String` in new
    `crafts.rs`): a creature the fort has seen (from `annals`/saga), a hero ("a statue of Kelda
    the Dragonslayer"), the monument, a building ("a figurine of a drawbridge"). Look shows it.
  - *Named halls (step 4, WLD-02).* `places.rs` writers (in new `naming.rs`): (a) when a hall is
    fitted at tier ≥ 1 or a Grand work is finished, name it after the fort's greatest hero or
    noble ("the Hall of Kelda Dragonslayer") or, failing that, `names::coin(Hall)`; (b) a deed
    inside a room's box renames it if heavier: a siege broken with its leader slain there ("the
    Hall of the Unbeaten", weight 7), three deaths in a day ("the Weeping Hall", 6), a beast slain
    ("the Hall of the Wyrm's Fall", 7), a wedding ("the Hall of Vows", 2). Hooks: one call
    `self.deed_at(pos, DeedKind, &who)` from `combat.rs` (leader slain, megabeast slain),
    `annals.rs` (siege end), `family.rs` (marriage), `game.rs: kill`. The gate gets a name from
    its first held siege ("Copper Gate"). Shown: look panel (W0-B line), map hover tooltip,
    chronicle (YearLog `#[serde(default)] named: Vec<String>` → "The great hall was named the
    Hall of the Unbeaten, for the goblins who broke there."), legends (`legend()`).
  - *Bay 12 extras (step 5):*
    - CRF-01 trophies: `combat.rs` death of a megabeast or siege leader in the fort spawns
      `ItemKind::Trophy` (label "the skull of Ushat the dragon"; the dragon's existing skull
      becomes one). `BuildingKind::Display` built from a trophy; `ov_adorn` sets displays in the
      main hall; passers think "admired the skull of …" (as `monument_daily`); if the foe's civ
      is a neighbour, its next siege says so ("they come for the skull of their general").
    - CRF-02 named weapons: `Item.kills: u16` (`#[serde(default, skip_serializing_if = zero)]`)
      counted in `heroes.rs: credit_kill`; at 5 kills the weapon is named `names::coin(Weapon)`,
      a legend and a saga line; its wielder thinks "is attached to Gutsplitter"; M5's A03 takes
      it further.
    - CRF-06 maker's marks: `Item.maker: Option<u32>` set at craft completion; item names add
      "(made by Urist)" for quality ≥ 3; ledger (O08 ECO-15) uses it.
    - CRF-08 name creator: `names.rs` (W0-B) used by all of the above.
    - CRF-13 jeweler: `Recipe::Encrust` at the Craftsdwarf, inputs `[Input::CutGem,
      Input::Furniture]` (new Inputs; Furniture accepts Table/Chair/Door/Cabinet/Chest/Statue/
      Figurine); output keeps kind and mat with `Item.decor: Option<Mat>` and value ×1.5; the
      overseer orders it at tier 3 when gems are in stock; Prosper raises it.
    - CRF-12 gems in engravings, engraved built walls: `desig_valid` allows Smooth/Engrave on
      constructed walls of stone or blocks; at tier 3, a masterful engraving consumes a cut gem
      within 20 tiles and marks the tile (high bit of `Tile.engraving`, no new tile field) for
      double value and "set with amethysts" in its description.
- **Steps:**
  1. *Main halls smoothed then engraved as the fort grows rich* · detail.rs (`ov_finish`), ov_m2.rs
     (`ov_tier`), blueprint.rs (`plan_halls`), overseer.rs `ov_labors` (one line), ambitions.rs
     (calls renamed) · test `detail.rs: a_rich_fort_smooths_then_engraves_its_halls`: dug-in
     Steady fort, give it wealth (spawn 40 gems at the wagon), run 30 days: ≥ 60% of the level-0
     spine tiles' walls smoothed, then (after YEAR/3, tier 2) some engraved.
  2. *Bedroom furniture and noble suites* · buildings.rs (kinds, recipes), items.rs (kinds),
     psyche.rs `building_value`, blueprint.rs (`furn`), overseer.rs `ov_fit` (Bedrooms extras,
     Suite arm), ov_m2.rs (`ov_furnish`, `ov_suites`) · tests `buildings.rs:
     cabinets_and_chests_are_made_and_placed` and `overseer.rs: a_noble_gets_a_suite` (make a
     dwarf Mayor in a dug-in fort, give wood/stone, run 20 days: a Suite room exists with
     Bedroom+Office+Dining zones owned by the mayor).
  3. *Statues at the heart, the monument in the atrium* · blueprint.rs (`features`), ov_m2.rs
     (`ov_adorn`), overseer.rs/ambitions.rs (`monument_spot`), crafts.rs (`subject`) · test
     `blueprint.rs: the_atrium_and_hub_have_a_heart` (features exist for Radial/Spokes) and
     `overseer.rs: statues_stand_in_the_atrium` (Radial fort, tier forced 2, statues spawned in
     stock: a Statue building on a feature spot within 10 days).
  4. *Halls named for heroes and deeds* · naming.rs (new: `deed_at`, `name_hall`), hooks in
     combat.rs/annals.rs/family.rs/game.rs (one line each), chronicle.rs (`named` lines) · tests
     `naming.rs: a_hall_takes_the_name_of_what_happened_in_it` (a fitted Dining room; a siege
     leader killed inside it via the combat death path → `place_at` gives a hall name, weight 7;
     a later wedding doesn't rename it) and `chronicle.rs: named_halls_go_in_the_chronicle`.
  5. *Trophies, named weapons, maker's marks, jeweler, gem engravings, statues of things* ·
     items.rs (`kills`, `maker`, `decor`, Trophy), buildings.rs (Display, Encrust, Inputs),
     heroes.rs `credit_kill`, crafts.rs, detail.rs (built walls, gems), combat.rs (trophies) ·
     tests `crafts.rs: a_weapon_that_kills_enough_gets_a_name`,
     `crafts.rs: a_trophy_goes_on_display_and_is_admired`, `crafts.rs: the_jeweler_sets_a_gem`,
     `detail.rs: built_walls_can_be_engraved`.
  6. *Done when* · test `overseer.rs: a_lived_in_fort` (one year, not three, to keep it under
     10 s: Builder on seed 5 with wealth seeded; assert smoothed spine, a Bedrooms room with
     Cabinet+Chest, a named place) plus the batch criterion (3-year survivors: ≥ 80% with
     smoothed halls and a named place).
- **Overseer & screen:** tiers are an overseer sense of the fort's standing; vain/pious push
  engraving sooner (tier thresholds ×0.7 for vain), greedy sells trophies instead of displaying
  them (a note). Notes: "Had the great hall smoothed: the fort can afford to look its best now."
  Screen: the map shows smoothed/engraved walls (existing glyphs), statues and displays (new
  glyphs), look panel names, chronicle lines, legends.
- **Measure:** `M2Stats.smoothed_pct` (share of level-0 hall tiles smoothed at the end) and
  `named_places`; batch targets as in the release check. Watch wealth: engraving raises room
  value and wealth, which grows sieges (S03 scales with wealth): survival must not drop.
- **Risks / open questions:** engraving labor pulls dwarves from hauling in small forts (only
  from tier 1, and 1 + pop/20); the Item struct grows (3 small fields; skip_serializing keeps
  saves small). New enum variants in Mat/ItemKind/BuildingKind are append-only (serde by
  name). Is "the Hall of <hero>" too repetitive? Names fall back to coined ones when the same
  hero already names a hall.
- **Parallel work:**
  - **Writes:** detail.rs (`ov_finish`, built walls, gems), crafts.rs (new), naming.rs (new),
    blueprint.rs (`furn`, `features`, `plan_halls`), buildings.rs (kinds, recipes, Inputs),
    items.rs (kinds, fields), psyche.rs (`building_value`), overseer.rs (`ov_fit` arms,
    `ov_labors` line, `monument_spot`), ambitions.rs (`ov_detail` call sites), ov_m2.rs (lines),
    chronicle.rs (`named`), one-line hooks in combat.rs, annals.rs, family.rs, game.rs.
  - **Reads / depends on:** W0-B places/names/saga; O02 step 2 (blueprint transform: `furn` and
    `features` must be transformed too, so land after O02's blueprint change); M1 nothing.
  - **Hot spots:** buildings.rs `BuildingKind`/`Recipe` enums (append at the end; O08, O11, O12
    also append: serialize enum edits by appending only, and resolve conflicts by keeping both);
    items.rs `ItemKind`/`Item` (append; O12 also adds Trinket); overseer.rs `ov_fit` (two new
    arms).
  - **Agent size:** split in three: O03-a (steps 1-3, 2 sessions, after O02), O03-b (step 4,
    names, 1 session, wave 1 right after W0-B), O03-c (step 5, crafts, 1.5 sessions, wave 1).

### O04 Defences in the plan  (effort: 4 sessions; risk: high)

- **Design:** the gatehouse becomes a staged project in `gatehouse.rs` (moved there in W0-B),
  advanced daily by `ov_gatehouse(o)` (called where `ov_defences` was). `Overseer.gate_stage:
  u8`: 0 gate (doors, bridge, lever, cage traps — today's code), 1 trap hall, 2 pit bridge and
  moat, 3 towers. Each stage starts when the previous is done and its trigger holds:

  | stage | trigger (default) | Warlord / cautious | needs |
  |---|---|---|---|
  | 1 trap hall | after the first siege or YEAR/2 | at once after stage 0 | 6 mechanisms, 2 weapons or bolts |
  | 2 pit bridge + moat | wealth ≥ 8,000 or 2nd siege | after stage 1, from day 120 | mechanisms 4, blocks 10, buckets 3 |
  | 3 towers | 3rd siege or wealth ≥ 20,000 | after stage 2 | blocks 40, a marksdwarf squad (S07) |

  (Reckless never builds past stage 0, as today.) All positions through `o.rel`.
  - *Trap hall (stage 1; ENG-19, ENG-04, ENG-06, ENG-08, ENG-01).* The corridor x+4..x+13 (local)
    is filled: cage traps x+4..5, `BuildingKind::Spikes` x+6..7 (new: upright spikes, built
    from a mechanism + 3 bolts or a spear; retracted until a linked lever or plate fires, then
    strike every creature on the tile for 3 rounds: ENG-04), weapon traps x+8..9 (existing kind,
    now loaded with spare weapons), stone-fall x+10..13 (existing). A
    `BuildingKind::PressurePlate` (ENG-06: `plate: PlateTrigger { Hostile, Citizen, Any,
    WaterDepth(u8) }`, `links` like a lever) at x+12 linked to the spikes; a `Tripwire` (ENG-08:
    a cheap thread-built trap that trips: the victim falls prone 30 ticks, so the traps behind
    hit). Jammed traps (ENG-01): `traps.rs: spring_trap` and `traps_tick` skip a trap whose tile
    holds ≥ 2 corpses or cages; after a siege the overseer marks those corpses for the refuse
    pile (`toggle_dump`) at priority, and a dwarf clearing a trap hall while foes remain is an
    "under fire" spotlight. Better traps (ENG-19): stone-fall traps keep firing until their
    boulder is spent (load count), weapon traps strike up to 3 times.
  - *Pit bridge and moat (stage 2; ENG-20, ENG-03, ENG-02).* At local x+15..16 (past the trap
    hall, before the halls' hub) the overseer channels a 2×2 pit two levels deep (`Desig::Channel`
    then `Dig` below), and builds a `Bridge` across it. New: a bridge may stand over open space
    (`can_place` allows Bridge on `Open` when a floor is beside it on the run; the building
    remembers `over_pit: bool`); lowered, the tile is walkable (`world.rs`: `t.door ==
    BRIDGE_DOWN_OVER_PIT` treated as floor by `walkable`/`astar`); raised, whoever is on it
    falls (`traps.rs: clear_tile` → new `fall(id, from)`: drop to the first floor below,
    1 wound per z, stunned 60 ticks; goblins in the pit can't climb out — the pit has no ramp).
    Ponds (ENG-03): `ZoneKind::Pond` over the pit's bottom; haulers carry water buckets from the
    nearest water (wells, river) and pour until depth 4 (job in new `ponds.rs`); a creature in
    depth ≥ 4 water drowns as dwarves do (`drown` counter on Creature). Deliberate flooding
    (ENG-02): when the site has a river at the gate's z ± 1, a floodgate on a lever lets it into
    the pit instead of buckets. Wide drawbridges (ENG-20): bridges may be placed in a row and
    linked as one ("a three-wide drawbridge"); the surface yard's gate (from Fortify) becomes
    one when stage 2 comes.
  - *Towers (stage 3).* At the surface yard (`rel_top`): two 3×3 towers of constructed walls with
    an up-stair inside and a fortification ring on the level above (constructed floor + 4
    fortifications), placed either side of the yard gate. The marksdwarf squad's station during a
    siege moves up there (`ov_alert` station spots come from `gatehouse::stations(o)`). This
    needs shots across z, which `military.rs: clear_shot` refuses (`a.z != b.z`): extend it to
    one level of height difference when the shooter stands on a tile next to a fortification.
  - *How the overseer uses it (step 2).* Bold and Warlord overseers *let them in*: the outer gate
    bridge waits (`patience` +200) while the pressure plate and pit bridge do the killing;
    cautious ones raise the gate at once (today's behaviour) and use the trap hall only if it's
    breached. The pit bridge is raised by its own lever when ≥ 2 hostiles stand on or beyond it
    (watched in `oversee_alert`, 10-tick cadence, cheap). Trigger tables scale with traits
    (`Trait::Cautious`/style Warlord → stages sooner).
  - *Into the legends (step 3).* `SiegeNote` gains `#[serde(default)] trapped: u32, fell: u32,
    drowned: u32, broke_on: Option<String>` (counted in `traps.rs` and `fall`). When a siege ends
    "routed"/"withdrew" with ≥ half of its foes killed by traps, pit or towers, `annals_tick`
    calls `defences_held(i)`: legend "The goblins of X broke upon the killing hall of Y", a
    spotlight, a milestone ("A siege broken on the defences"), the gate place renamed (via O03's
    `deed_at`), and O09 names the episode.
- **Steps:**
  1. *The gatehouse grows: trap hall, pit bridge, towers* — three commits:
     1a trap hall with spikes, plates, tripwires, jamming · gatehouse.rs, traps.rs, buildings.rs
        (Spikes, PressurePlate, Tripwire) · tests `traps.rs: a_pressure_plate_fires_the_spikes`,
        `traps.rs: a_jammed_trap_does_not_fire_until_cleared`, `gatehouse.rs: the_trap_hall_is_built_after_the_gate`.
     1b pit bridge and pond · gatehouse.rs, traps.rs (`fall`), world.rs (`walkable` for bridges
        over pits), ponds.rs (new), creature.rs (`drown`) · tests `traps.rs:
        raising_the_pit_bridge_drops_whoever_is_on_it`, `ponds.rs: haulers_fill_a_pond`,
        `path.rs: a_lowered_bridge_over_a_pit_is_walkable`.
     1c towers · gatehouse.rs, military.rs (`clear_shot` height), construct use · tests
        `military.rs: a_marksdwarf_shoots_down_from_a_tower`, `gatehouse.rs: towers_rise_at_the_yard`.
  2. *Warlord and cautious overseers build them sooner, and use them differently* · gatehouse.rs
     (trigger table, let-them-in tactic), overseer.rs `ov_alert` (patience from gatehouse) ·
     test `gatehouse.rs: a_warlord_has_a_moat_and_trap_hall_before_its_third_siege` (the
     Done-when, scripted: Warlord fort, `force_siege` twice with stocks of mechanisms/blocks
     seeded; before the third `force_siege`, `gate_stage ≥ 2` and the pit exists) and
     `gatehouse.rs: a_bold_overseer_lets_them_into_the_killing_hall` (scripted siege: the outer
     bridge stays down longer than for a cautious one, and traps fire).
  3. *A siege that breaks on the defences goes into the legends* · annals.rs (fields,
     `defences_held`), traps.rs counters · test `annals.rs: a_siege_broken_on_the_traps_is_legend`.
- **Overseer & screen:** gate stage is part of the overseer's intent ("Next: the pit bridge,
  once 10 blocks are cut"); notes per stage; the Fortify ambition (`ambitions.rs: pursue`)
  advances the next stage instead of only stone traps/yard; `ambition_met(Fortify)` = stage
  advanced. Screen: new glyphs (spikes `↑`, plate `^`, tripwire `-`, pit water), the ghost plan
  (O02) shows the planned gatehouse, a spotlight when the pit bridge drops foes, chronicle and
  legends lines.
- **Measure:** `M2Stats.gate_stage`, traps/foes ratio from SiegeNote (`trapped`, `fell`,
  `drowned`); batch target: Warlord forts reach stage 2 before their third siege in ≥ 60% of
  cases; foes slain per siege doubles vs M1; dwarves lost in later sieges falls; no rise of
  deaths "inside the shelter".
- **Risks / open questions:** high: the pit bridge needs walkability changes in the path search
  (hot path: one extra branch in `walkable`); falling creatures are a new mechanic; ponds are a
  hauling job; a bridge over a pit that's raised during peacetime traps dwarves below (only
  raise with hostiles on it, and never with a citizen on it). The let-them-in tactic could make
  Warlords lose more forts: tune with the batch (a knob in S10's file). Do we want dwarves to
  throw prisoners in the pit (ENG-03's other half)? Proposal: later, with M3 justice.
- **Parallel work:**
  - **Writes:** gatehouse.rs (most), traps.rs (`spring_trap`, `traps_tick`, `fall`, plates),
    ponds.rs (new), buildings.rs (3 kinds), world.rs (`walkable` branch), creature.rs (`drown`),
    military.rs (`clear_shot`), annals.rs (SiegeNote fields, `defences_held`), overseer.rs
    `ov_alert` (patience, stations: 2 lines), ambitions.rs (Fortify arm).
  - **Reads / depends on:** M1 S04 (gate parts), S07 (marksdwarves for towers), W0-A (`rel`),
    W0-B (gatehouse.rs move), O03-b `deed_at` (soft: rename the gate if present).
  - **Hot spots:** world.rs `walkable` (one branch: coordinate with anything touching path
    performance), overseer.rs `ov_alert` (two lines), buildings.rs enum (append).
  - **Agent size:** too big for one session: O04-a (1a + 2's trigger table, 1.5 sessions), O04-b
    (1b pit/pond/fall, 1.5 sessions, after O04-a), O04-c (1c towers + 3 legends, 1 session).

### O05 Spectator mode  (effort: 5 sessions; risk: medium)

- **Design:**
  - *Mood strip (UI-01).* When `App.mood_strip` (default on while the overseer plays, `U`
    toggles), `ui.rs: draw` reserves a second top row: one cell per living citizen sorted by
    stress, `☺` green → yellow → red by stress band, `☹` for tantrum, `Ω` mad, `♦` in a fey mood,
    `†` for those dead in the last 3 days (dim). Clicking one: `Target::Dwarf(id)` (new Target
    variant) centres the camera and opens the unit sheet. Built in `ui_units.rs:
    draw_mood_strip`.
  - *Director camera (UI-03).* `App.director: Director { on: bool, follow: Option<u32>, locked:
    bool, last_cut: u64, why: String }` (new `src/director.rs`). `Game::drama_spots(&self) ->
    Vec<(i32, Pos, String)>` (game side, cheap, in new `game/drama_spots.rs`): fights (dwarves
    with `attacker` in the last 20 ticks; score by hp lost), active sieges near the gate, fires,
    tantrums and madness, a mood at work, a monster within 30 of the halls, a festival. Every 10
    frames, if not locked and ≥ 8 s since the last cut, the camera cuts to the best spot when it
    beats the current by a margin; `follow` keeps a dwarf centred (and changes z with them). `V`
    toggles the director, `F` follows the dwarf under the cursor (or unfollows), `Shift+F`
    locks. The status bar shows `◉ a fight by the gate`.
  - *Sound reports (UI-06).* With `LogEntry.at`/`kind` (W0-B), app-side: a combat/death/collapse
    entry whose `at` is off-screen or on another z is shown as a sound: "Screams from the deep,
    far to the east" / "The clash of steel near the depot" (place name from `place_at` if any);
    Enter on the announcements jumps there. Game side: `combat.rs` blows and deaths, `disasters.rs`
    cave-ins and `fire.rs` use `msg_at`.
  - *Last words (UI-02).* `game.rs: kill` calls `fn last_words(&self, d: &Dwarf, cause: &str) ->
    Option<String>` (new `game/lastwords.rs`): templates (original) keyed on what the dwarf
    cares about most: spouse/children ("Tell Kib the ale's on me"), a grudge or the killer
    ("Mind the goblin with the spear"), their craft or a named weapon, their god or stress (mad:
    a ramble), a hero's boast; picked with `names::hash` (no rng). Skipped for instant deaths
    (crushed, drowned at once) unless a hero. Goes to: the announcement, `DeathNote.last_words`
    (`#[serde(default)]`), the hero's spotlight card, the saga, and O06's death scenes.
  - *Announcement control (UI-07).* `App.announce: Announce` loaded from `saves/announce.json`
    (app setting, not in the game save): per `LogKind` {pause, recenter, show}, defaults matching
    today (deaths and sieges pause via spotlights). `Mode::Announce { sel }` from the reports
    screen (`a`). `app.rs: update` honours pause/recenter for new log entries since the last
    frame; `ui.rs` filters hidden kinds. Building completions (new `msg_at` in the build-complete
    path, kind Building, hidden by default).
  - *Step size (UI-32).* While paused: `s` advances one tick, `S` one hour (DAY/24 = 50 ticks),
    `D` one day (all three are free in `key_normal`). `App.step_pending: u32` consumed in
    `update` at full speed; flash "stepped 50 ticks".
  - *Highlight reel (UI-04).* `game/reel.rs` (new): `Reel { clips: Vec<Clip> }` kept in
    `Game.reel` (`#[serde(default)]`, also copied into the ruin save). `Clip { tick, title, text,
    at: Pos, weight: u8, frames: Vec<Frame> }`, `Frame { tick, w: u8, h: u8, cells: Vec<(char,
    [u8;3])> }`, 31×13 cells via `glyphs::tile_look` + `unit_glyph`. Started by `spotlight()`
    (one line: `self.reel_start(title, text, at, weight)`) and by every death during a siege
    (merged into the siege clip) or of a hero; `m2_tick` captures one frame every 20 ticks for
    active clips until 8 frames (≈ 3 KB a frame raw; frames are run-length encoded on save). At
    most 60 clips: the lowest weight goes first. `Mode::Reel { clip, frame }` plays at 4 frames
    a second with the card text under it, ←→ between clips; opened from the game-over page
    (`h`), the chronicle (`h`), and *Watch a fort* when the fort falls.
  - *Export (UI-10).* `E` writes `exports/<fort>-<year>/` (new `src/export.rs`): `map-z<N>.txt`
    for every level from the surface to the deepest dug (glyphs.rs), `fort.html` (all levels as
    coloured `<pre>` blocks with a level picker, inline CSS, no scripts needed), and `story.md`
    (chronicles, deaths with last words, sieges by name, the overseer's journal, legends of the
    fort). `--export DIR` on autoplay writes one per fort.
- **Steps:**
  1. *Mood strip* · ui_units.rs, ui.rs `draw` (row split), app.rs (`U`, Target) · test `ui.rs:
     the_mood_strip_shows_every_dwarf` (dump: one glyph per citizen; a dwarf with stress 300 red).
  2. *Director camera* · director.rs, game/drama_spots.rs, app.rs `update` (one call), keys ·
     tests `drama_spots.rs: a_fight_is_the_most_dramatic_thing` and `director.rs:
     the_camera_cuts_to_the_fight_and_follows_a_dwarf` (App test: spawn a hostile next to a
     dwarf off-screen; after a few frames the view contains it; `F` keeps a walking dwarf
     centred).
  3. *Sound reports* · combat.rs/disasters.rs/fire.rs `msg_at`, app/ui announcements · test
     `ui.rs: a_far_fight_is_heard_not_seen`.
  4. *Last words* · lastwords.rs, game.rs `kill` (one line), annals.rs (field), heroes.rs card ·
     test `lastwords.rs: the_dying_speak_of_what_they_love` (a married dwarf mentions the spouse;
     same death → same words).
  5. *Announcement control* · app_m2.rs (`key_announce`), ui_overseer.rs or ui_units.rs draw,
     app.rs `update` · test `app_m2.rs: hidden_announcements_dont_show_and_paused_ones_pause`.
  6. *Step size* · app.rs (keys, `step_pending`) · test `app.rs: step_one_tick_and_one_hour`.
  7. *Highlight reel* · reel.rs, spotlight.rs (one line), m2hooks.rs (one line), ui_reel.rs,
     game-over page key · tests `reel.rs: a_spotlight_starts_a_clip_of_eight_frames`,
     `reel.rs: the_reel_keeps_the_weightiest_sixty`, and the Done-when
     `overseer.rs: an_autoplay_fort_leaves_a_reel_of_its_sieges_and_deaths` (`#[ignore]` for
     3 years; the non-ignored version: 1 year with `force_siege` at day 100: a clip titled for the
     siege with 8 frames, and one for a death).
  8. *Export* · export.rs, app.rs `E`, main.rs `--export` · test `export.rs:
     a_fort_exports_its_levels_and_story` (temp dir: files exist, the html contains the fort's
     name, every dug level has a txt).
- **Overseer & screen:** the overseer's spotlights carry its commands (O01) and become clips;
  its notes about deaths quote last words (sentimental). Everything here is screen: strip,
  camera, sounds, cards, reel, export.
- **Measure:** `M2Stats.clips` per fort (target ≥ 1 per siege + 1 per hero death); frame time
  under F3 with director and strip on stays within 10% of off; save size growth ≤ 1.5 MB for a
  3-year fort (check with the `profile_a_save` ignored test).
- **Risks / open questions:** reel size in saves (RLE, 60-clip cap); the director must not fight
  the player (it only runs while watching, or when `V` is on); `S`/`D` keys collide with nothing
  in Normal (checked: `S` only in ConfirmQuit). Export HTML with no scripts keeps it simple.
- **Parallel work:**
  - **Writes:** new director.rs, drama_spots.rs, lastwords.rs, reel.rs, export.rs, ui_units.rs,
    ui_reel.rs, app_m2.rs functions; one-line hooks in game.rs `kill`, spotlight.rs, m2hooks.rs;
    `msg_at` swaps in combat.rs, disasters.rs, fire.rs; app.rs keys and `update` (3 small
    blocks); ui.rs `draw` (row split).
  - **Reads / depends on:** W0-B (LogEntry at/kind, glyphs.rs, saga, Mode placeholders,
    m2hooks).
  - **Hot spots:** app.rs `update` and `key_normal` (several packages add keys: keep each to one
    arm calling into app_m2.rs); ui.rs `draw` (the row split is one edit; do it in O05-a).
  - **Agent size:** O05-a (steps 1-3, 2 sessions), O05-b (4-6, 1.5 sessions), O05-c (7-8,
    1.5 sessions); all three can run in parallel (disjoint new files; coordinate app.rs keys).

### O06 Know your dwarves  (effort: 3 sessions; risk: low)

- **Design:**
  - *Life story (UI-05).* `Mode::Unit { id, sel }` gains tabs via a new field `tab: usize`
    (constructors updated: few). Tabs: Now (today's `unit_view`), Thoughts (the 8 thoughts with
    causes and age), Life (from `saga_of(id)`, grouped by season and year: "Spring 203 · arrived
    with the second migrants · Summer 203 · married Kib · …"), Family, Health. Drawn in
    `ui_units.rs: unit_tabs`. Saga writers beyond W0-B's: first masterwork, joined a squad,
    wounded badly (hp < half), recovered from melancholy, duel won, festival feat, expedition.
  - *Unit list with stress (UI-13).* `Mode::Units { sel }` gains `cat: u8, sort: u8`: categories
    Citizens / Pets and livestock / Visitors / Dead and missing; sort by stress, happiness, job,
    name (`s` cycles); a stress bar per row.
  - *Live unit sheet (UI-14).* The unit sheet joins the dockable panes (`app.rs: dockable`), so
    the map stays live beside it, and `f` there follows the dwarf (O05 director's `follow`); Look
    with several things on a tile already cycles with Tab — Enter opens the sheet of the selected
    one.
  - *Family tree (UI-15).* `Mode::Family { id, sel }`: grandparents, parents, siblings, spouse,
    children, grandchildren from `Dwarf.parents`/`spouse` (computed by scan; ≤ 200 dwarves),
    pets owned (pets.rs), with alive/dead colour; Enter jumps to the selected relative's sheet;
    `z` zooms the camera to them.
  - *Death scenes in the chronicle (UI-08).* `chronicle.rs: write_chronicle` adds one line per
    notable death this year (hero, noble, soldier, parent, or anyone whose death was spotlit),
    from `annals.deaths` filtered by year: "Urist Copperhand, a mason of forty, was struck down
    by a goblin at the Copper Gate during the siege of the Black Spears; her last words were
    …". Function `annals.rs: death_scene(&DeathNote, &Game) -> String`.
  - *Status readouts (UI-35).* Health tab: each part with a severity colour (`body.part_status`
    ranks: bruised yellow, broken orange, mangled red, missing grey), bleeding rate, next
    treatment if S02's medicine has one (`medicine.rs` exposes `fn next_treatment(&Dwarf) ->
    Option<&str>`); a status tab "Animals and patients": pasture animals (hunger, tame,
    pregnant) and patients (bed, wounds, treatment).
- **Steps:**
  1. *Unit sheet tabs and the life story* · ui_units.rs, app.rs (Mode::Unit tab), saga writers
     (one line each in military.rs, duels, festivals, psyche) · test `ui.rs:
     a_dwarfs_sheet_tells_their_life` (dwarf with arrival, marriage and an epithet: the Life tab
     shows all three under their seasons).
  2. *Unit list categories and stress sort* · ui.rs Units draw, app.rs keys · test `ui.rs:
     the_unit_list_sorts_by_stress_and_lists_the_dead`.
  3. *Live sheet beside the map* · app.rs (`dockable`), ui.rs pane list · test `ui.rs:
     the_unit_sheet_opens_beside_a_live_map`.
  4. *Family tree screen* · ui_units.rs `draw_family`, app_m2.rs `key_family` · test `ui.rs:
     a_family_tree_shows_parents_spouse_and_children` (the Done-when, with step 1).
  5. *Death scenes in the chronicle* · chronicle.rs, annals.rs `death_scene` · test
     `chronicle.rs: notable_deaths_are_told_in_the_chronicle`.
  6. *Status readouts* · ui_units.rs, medicine.rs (accessor), status tab · test `ui.rs:
     injuries_are_coloured_by_how_bad_they_are`.
- **Overseer & screen:** the overseer reads the stress list: `ov_m2 → ov_watch_stress` (daily)
  finds the three most stressed; a sentimental or cheerful overseer gives them a break (sets
  `break_until` for a tavern visit if a tavern exists) and notes it ("It sent Urist, who was close
  to breaking, to the tavern"); others note only. Screen: the sheets, the tree, the chronicle.
- **Measure:** UI only; batch: none, except that `ov_watch_stress` must not cut survival (it
  costs labour); count tantrums per fort (report) to see whether it helps.
- **Risks / open questions:** `Mode::Unit` is constructed in several places (look, units list,
  click targets): adding `tab` touches them all once. Overlap with M3 C01 (inner life): C01
  should write its memories into the saga, not a new store.
- **Parallel work:**
  - **Writes:** ui_units.rs, app_m2.rs, chronicle.rs (`write_chronicle` death lines),
    annals.rs (`death_scene`), medicine.rs (accessor), ov_m2.rs (one line), one-line saga
    writers in military.rs, drama.rs, festivals.rs, psyche.rs.
  - **Reads / depends on:** W0-B saga; O05 step 4 `DeathNote.last_words` (soft: omit if absent).
  - **Hot spots:** app.rs `Mode::Unit` constructors (one-field edits), ui.rs Units draw.
  - **Agent size:** one agent, 2 packages: O06-a (1-3), O06-b (4-6); can run in parallel with
    O05 if app.rs edits are kept to the listed arms.

### O07 A mortal overseer  (effort: 4 sessions; risk: medium)

- **Design:**
  - *The overseer is a dwarf (OVR-02).* `Overseer.dwarf: Option<u32>` (W0-B). On hand-over
    (`set_overseer`) it embodies the expedition leader (`Noble::Leader`, else the most sociable
    adult); its `name` becomes theirs; if it has no traits yet they come from the dwarf's
    `Personality` (`fn traits_of(&Personality, &mut Rng) -> Vec<Trait>`: anxiety > 20 → Cautious,
    < −20 → Bold, artistry > 25 → Vain, sociability > 25 → Cheerful, diligence > 25 → Stubborn,
    temper < −25 → Sentimental; fill up to two at random; no clashes). Old saves: `dwarf` None →
    embodied at the next oversee. The overseer dwarf keeps doing ordinary work (it's a dwarf),
    never joins a squad (already excluded as a noble).
  - *Succession.* `ov_m2 → ov_reign(o)` (daily, and immediately from `kill` via a flag
    `Game.ov_fell: Option<(u32, String)>` set in `kill` when the dead dwarf is the overseer):
    if the overseer dwarf is dead, insane, away and missing on an expedition for more than a
    season, jailed (`jailed_until`), or deposed (faith, below): `succeed(o, end: ReignEnd)`.
    Heir: the Mayor if any (and not the cause), else the adult with the most standing (`epithet`
    rank × 10 + sociability + sum of regard / 10 + noble bonus). New traits from the heir's
    personality; if they equal the old pair, swap the second (the Done-when needs "different").
    `Reign { name, dwarf, traits, from, to, end: ReignEnd { Died(cause), Mad, Lost, Deposed,
    Elected } }` pushed to `o.reigns`. Effects: a spotlight "The overseer has fallen", a legend,
    a saga Overseer event, the chronicle (`YearLog #[serde(default)] reigns: Vec<String>`:
    "The reign of Urist the Cautious ended when goblins cut him down at the gate; Kib, bold and
    greedy, took up the ledger."), faith reset to 55, nudges cleared, current ambition dropped
    (the new overseer chooses its own). Mayor election (`psyche.rs: nobles_daily`): if the
    elected mayor isn't the overseer, the overseer is succeeded by the mayor (`Elected`), unless
    faith ≥ 60, in which case the overseer *is* elected (the pick is weighted).
  - *Faith (OVR-18).* `o.faith: i32` 0..=100 (start 60), changed by `fn faith(&mut self, o, by,
    why)` which logs to `o.faith_log` (last 12). Daily inputs: each dwarf death −3 (−6 if inside
    the shelter: "a broken promise"), Food or Drink trouble open −1/day, a siege routed +8 (withdrew
    +3), a festival +3, an artifact +2, the monument +10, an ambition met +3 (in
    `ov_ambition` when `ambition_met`), a command refused by a stubborn overseer −1, an inspired
    blueprint ignored −5 (O12). Thresholds (knobs): < 40 grumbling (dwarves get "grumbled about
    the overseer" +5 stress weekly; notes); < 25 defiance: each `boost` and labor change has a 30%
    chance to be ignored ("The masons would not cut blocks for it"), and a food riot when also
    hungry: up to 3 stressed dwarves tantrum at the food stockpile (`psyche.rs:
    tantrum_mischief`); < 10 for 10 days: deposition (`ReignEnd::Deposed`). A sense in the brain:
    `Sense::Doubt` (fires as `(50 - faith)/50` clamped) pushes Revel +1.2 and Provision +0.8, Venture
    −1.0 and Architect −0.8 (an unpopular overseer tries to win them back).
  - *Traits that drift (OVR-10).* `fn drift(&mut self, o, cause: Drift)` with `Drift { Massacre,
    Victory, Famine, Artifact, Monument, Loss(dwarf) }`, checked in `ov_reign` from the saga since
    the last check: a siege that cost ≥ 3 lives → a Bold overseer turns Cautious (50%), a
    Cheerful one Sentimental; a siege routed with no losses → Cautious turns Bold (40%); famine
    (hunger sense ≥ 0.7 for 10 days) → Cheerful turns Greedy; an artifact → +Pious if it has a free
    slot; the monument → Vain. At most once a season; recorded in the reign and the journal; note
    "The massacre at the gate broke its nerve: no longer bold, it is cautious now."
  - *Spin (OVR-5).* When a siege ends with `dwarves_lost > foes_slain` or the outcome is
    "withdrew" after losses, a vain overseer writes it up: the announcement and the chronicle's
    siege line use a spun text ("A glorious stand: the goblins were driven from our gate") and
    the true line is kept in `annals` (the report and the game-over page tell the truth). Faith
    +4; each dwarf who was in the fort during the siege and has anxiety < 0 or a dead friend gets
    "doesn't believe the overseer's tale of the siege" (+12 stress) and faith −1 each (capped −6).
    `YearLog #[serde(default)] spun: Vec<(String true_line, String spun_line)>`.
  - *The journal (OVR-06).* `Game::note_ov` also appends to `o.journal: Vec<JournalPage { year,
    reign: u32, lines: Vec<(u64, String)> }>` (no cap; a 3-year fort writes ~1,500 lines,
    ~100 KB). `J` opens `Mode::Journal { year }`, a parchment page like the chronicle
    (`ui_overseer.rs: draw_journal`), one page per year and reign. When the fort falls the journal
    is kept with the ruin save; `Overworld` gets `#[serde(default)] lost_journals: Vec<(i32 x,
    i32 y, String fort, Vec<String> last_page)>` so M8's P01 can let a later fort find it
    (expeditions to that ruin — hook only now).
- **Steps:**
  1. *The overseer is a dwarf, and is succeeded when it falls* · new `game/reign.rs` (`embody`,
     `ov_reign`, `succeed`, `traits_of`), overseer.rs `set_overseer` (embody), game.rs `kill`
     (one line), psyche.rs `nobles_daily` (election), chronicle.rs (`reigns`) · test
     `reign.rs: an_overseer_killed_in_a_siege_is_succeeded` (the Done-when: Steady on seed 5,
     a DAY in, traits forced `[Bold, Vain]`, `force_siege`, kill the overseer dwarf with
     `kill_for_test(…, "has been struck down")`; after the next oversee: another dwarf, another
     name, traits differ, `reigns.len() == 2`, and `write_chronicle_for_test` gives a line with
     "reign").
  2. *Faith in the overseer* · reign.rs (`faith`, thresholds, riots, deposition), ambitions.rs
     (`Sense::Doubt`), troubles.rs/`ov_labors` (defiance check: one helper `self.obeyed(o) ->
     bool`), ui_overseer.rs (faith bar and log on the Autoplay screen), ui.rs status bar (`faith
     54`) · tests `reign.rs: deaths_in_the_shelter_cost_faith_and_a_rout_restores_it`,
     `reign.rs: an_overseer_without_faith_is_deposed`.
  3. *Traits that drift* · reign.rs `drift` · test `reign.rs: a_massacre_makes_a_bold_overseer_cautious`
     (seeded with an rng where the 50% roll passes, or loop seeds until it does, asserting it
     happens in ≥ 1 of 5 and never twice in a season).
  4. *Spin* · reign.rs (`spin_siege`), annals.rs (call at siege end), chronicle.rs (`spun`) ·
     test `reign.rs: a_vain_overseer_spins_a_defeat_and_some_dont_believe_it`.
  5. *The journal* · reign.rs or new journal.rs, overseer.rs `note` (one line), ui_overseer.rs,
     app_m2.rs (`J`), overworld.rs (`lost_journals`) · test `journal.rs:
     every_note_goes_in_the_years_page` and `ui.rs: the_journal_opens_beside_the_chronicle`.
- **Overseer & screen:** this item *is* the overseer as a person: reigns on the Autoplay screen
  ("The third overseer of Copperhall"), the faith bar and why it moved, the brain's Doubt
  sense, notes about drift, spun announcements, the journal. Batch: `M2Stats.reigns`,
  `faith_end`; the report lists how reigns ended.
- **Measure:** batch: ≥ 1 succession in 20-50% of forts (too many means faith is too twitchy);
  depositions < 10% of forts; survival unchanged within noise (defiance costs work); spin in ~half
  the vain overseers' lost sieges.
- **Risks / open questions:** the overseer dying takes its plan state with it? No: the
  `Overseer` struct (rooms, plan, gate) carries over; only person, traits, ambition and nudges
  change. The player's chosen traits (O01) are overwritten at succession: say so on the trait
  screen. Overlap with M8 P02 (crown and succession) and M3 C06 (the council): they should reuse
  `Reign`/faith, not add their own. Should the player's `A` take over end the reign? Proposal: the
  overseer steps aside (no reign end) and resumes when handed back.
- **Parallel work:**
  - **Writes:** reign.rs and journal.rs (new), ambitions.rs (`Sense::Doubt` + one link block),
    overseer.rs (`set_overseer`, `note`), game.rs `kill` (one line), psyche.rs `nobles_daily`
    (election branch), chronicle.rs (YearLog fields, lines), annals.rs (spin call), ui_overseer.rs,
    app_m2.rs, overworld.rs (one field).
  - **Reads / depends on:** W0-B (Overseer fields, saga, Mode::Journal, `ov_m2`).
  - **Hot spots:** ambitions.rs `Sense` enum (O12 adds `Sense::Inspired`, O10 may add
    `Sense::Ore`: add all three senses in W0-B with zero links, so the enum isn't edited in
    parallel); chronicle.rs YearLog (O03, O06, O07, O09 all add lines: give each its own
    function `fn lines_<item>(&self, log) -> Vec<String>` called from `write_chronicle`, one
    call each).
  - **Agent size:** O07-a (steps 1-2, 2 sessions), O07-b (3-5, 2 sessions, after O07-a).

### O08 Orders that think  (effort: 6 sessions; risk: high)

- **Design:**
  - *Where inputs come from (ECO-11).* New `game/supply.rs`:

    ```rust
    pub enum Source { Craft(Recipe), Mine(MatFilter), Chop, Farm, Hunt, Fish, Butcher, Buy }
    pub enum MatFilter { Ore(Option<Mat>), Flux, Gem, BuildingStone }
    /// What makes something `input` accepts, best first (an explicit table:
    /// Fuel → Craft(Charcoal); Bar/IronBar → Craft(SmeltOre) with Ore(Hematite) for iron;
    /// Flux → Mine(Flux); Ore → Mine(Ore); Log → Chop; Thread → Craft(SpinThread); Cloth →
    /// Craft(Weave); Leather → Craft(Tan); Hide/Bone/Tallow/Carcass → Hunt|Butcher;
    /// Brewable/Grain/Edible/Fiber → Farm; RoughGem → Mine(Gem); Ash → Craft(Ash); …)
    pub fn sources(input: Input) -> &'static [Source];
    impl Game {
        /// How many of `input` the fort has free.
        pub fn have(&self, input: Input) -> usize;
        /// Make sure `n` jobs of `recipe` can be fed: queue child orders for missing
        /// inputs, ask for gathering, and return what is still waited on.
        pub(super) fn feed(&mut self, recipe: Recipe, n: u32, depth: u8) -> Vec<Want>;
    }
    pub struct Want { pub input: Input, pub via: Source, pub queued: bool }
    ```

    `WorkOrder` gains (`#[serde(default)]`): `mat: Option<Mat>` (ECO-34), `cond: Cond`
    (ECO-04), `parent: Option<Recipe>`, `until: u64`, `waiting: Vec<Input>` (for the screen).
    `orders.rs: manage_orders` calls `feed` for each order whose stock is short; `feed` recurses
    (depth ≤ 3: steel → iron bars → ore + charcoal → logs) adding child orders `parent: Some(r)`
    with `keep` raised to what the parent needs, and *gather requests*: `Mine` → vein designation
    on the nearest known matching tile (O10's `follow_vein`, or `ov_mine`'s scan as a fallback);
    `Chop` → a chop designation ring like `troubles.rs: fix(Wood)`; `Hunt` → `self.hunt = true`;
    `Farm` → nothing but a trouble (the overseer handles fields). `game.rs: find_craft`'s warning
    becomes a one-per-season note keyed on (shop, input) (`warns` key without the daily repeat:
    new `warn_once(key, text, every)`), and the order's `waiting` shows on the orders screen
    ("Steel: waiting for flux (miners sent), charcoal (queued)").
  - *Conditional orders (ECO-04).* `Cond { Keep(u32) (default; today's `keep`), Below { kind:
    ItemKind, n: u32 }, After(Recipe), Every { ticks: u64, times: u32, done: u32 } }`;
    `orders.rs: due(&WorkOrder) -> u32` turns each into a number of jobs to queue now. The
    orders screen (`Mode::Orders`) gets `c` to cycle the condition and `a` "auto" (fills
    sensible conditions: food/drink Below 2×pop, furniture Below pop/2, weapons Keep squad
    size). The overseer writes its plans as orders: `ov_orders` uses `Below` for stocks, `Every`
    for seasonal crafts for trade, and the troubles' `boosts` become orders with `until` (the
    `boosts` list remains for old saves and is migrated into orders on load).
  - *Material-specific orders (ECO-34).* `mat: Some(m)` restricts `find_craft`'s
    `input.accepts(it)` to `it.mat == m` (for the first input); order screen picks the mat from
    the stock (`m` cycles materials in stock). Noble demands (`positions.rs: Demand`) and mandates
    name materials, so:
  - *Mandates feed orders (ECO-35).* `psyche.rs: issue_mandate` → when the overseer plays (or the
    player toggled "mandates to orders" on the nobles screen), `add_order_for(kind, mat, n)`
    finds the recipe producing `kind` (`orders.rs: recipe_for(kind, mat)`), with `Cond::Keep(n)`
    and `until` = mandate deadline.
  - *Masters and apprentices (ECO-13).* `Building.master: Option<u32>`, `min_skill: u32`
    (`#[serde(default)]`); `find_craft` skips dwarves below `min_skill` (unless nobody else could
    — never stall); a dwarf working at a shop whose master is someone else gains XP ×1.5 if the
    master is within 10 tiles (an apprentice). Query screen shows and sets them (`M` master =
    selected worker, `+/-` min skill). The overseer appoints the most skilled as master of each
    workshop (`ov_m2 → ov_masters`, weekly).
  - *Job priorities and postings (ECO-20).* Sparse priorities: `Game.prio: HashMap<Pos, u8>`
    for designations (default 4) and `Order.prio: u8` (`#[serde(default = four)]`) for workshop
    jobs. Job search (`game.rs: find_job` ordering): candidate jobs sorted by (prio, distance);
    every 50 ticks a dwarf on a prio ≥ 5 job drops it if a prio ≤ 2 job of a labor they have is
    unclaimed (an "urgent" pull). Designate mode: `1`-`7` set the priority for new designations.
    The overseer uses 2 for the gate, shelter and rescue digs, 6 for detailing.
  - *Vein mining (ECO-14):* implemented in O10 (DIG-11); ticked here when O10 lands.
  - *Records of production (ECO-15).* `Game.ledger: Ledger { made: BTreeMap<(u32 year, u32
    dwarf, ItemKind), u32>, mined: BTreeMap<(u32, Mat), u32> }` written at craft completion and
    dig completion; status tab "Ledger" (top makers, this year and all time); chronicle line
    ("Our smiths forged 14 axes, 9 of them by Kib"); game-over page; `Item.maker` (O03) shown.
  - *Tooltips (ECO-42).* The orders pick list and the workshop's add-task list show "makes 4
    blocks · needs a boulder" (`recipe.product()`, `input_desc_at`).
- **Steps:**
  1. *Orders queue their own inputs* · supply.rs, orders.rs (`manage_orders`, WorkOrder fields),
     game.rs `find_craft` (warning → `warn_once`, `waiting`) · tests `supply.rs:
     an_order_for_steel_queues_iron_charcoal_and_flux` (the Done-when part 1: a fort with
     smelter, wood furnace, forge, a manager, hematite and limestone known, no bars: add a steel
     order; after one `manage_orders` the orders hold SmeltOre (parent MakeSteel) and Charcoal,
     and a dig designation sits on hematite and limestone), and `supply.rs:
     no_announcement_repeats_a_missing_input` (10 days with flux unobtainable: ≤ 1 announcement
     about it).
  2. *Conditional orders* · orders.rs (`Cond`, `due`), app.rs/ui.rs orders screen · test
     `orders.rs: below_after_and_every_conditions_queue_when_due`.
  3. *Material-specific orders and mandates to orders* · orders.rs (`recipe_for`), game.rs
     `find_craft` (mat filter), psyche.rs `issue_mandate` · tests `orders.rs:
     a_granite_table_order_uses_granite`, `orders.rs: a_mandate_becomes_an_order`.
  4. *The overseer plans with orders* · overseer.rs `ov_orders` (conditions, migrate boosts),
     troubles.rs `boost` → orders · test `troubles.rs: the_overseer_sees_to_what_the_fortress_lacks`
     still passes, plus `orders.rs: the_overseers_orders_carry_conditions`.
  5. *Masters and apprentices* · buildings.rs (fields), game.rs `find_craft` (min skill, XP),
     ov_m2.rs (`ov_masters`), ui query · test `orders.rs: an_apprentice_learns_faster_by_a_master`.
  6. *Job priorities* · game.rs job search, designate keys · test `jobs.rs:
     an_urgent_dig_pulls_a_miner_off_a_minor_one` and the `one_year_timing` check (must stay
     within 10%).
  7. *Ledger and tooltips* · new ledger.rs, craft completion hook, ui status tab, orders screen ·
     tests `ledger.rs: who_made_what_is_written_down`, `ui.rs: render_work_orders` updated.
- **Overseer & screen:** the overseer stops fussing over inputs (troubles fall) and writes plans
  as orders ("Ordered steel: first iron from the hematite to the north, charcoal, and flux");
  masters appointed; priorities on its digs. Screen: orders screen with conditions, materials and
  what each waits on; query master/min skill; ledger tab; chronicle lines.
- **Measure:** `M2Stats.chained_orders`; the report counts announcements per fort and repeated
  "needs" lines (target: none repeated within a season); `troubles` noted per fort falls by half;
  weapons/armour in stock at the first siege (from TRIAL_SIEGE) up.
- **Risks / open questions:** high: recursion can queue junk (depth ≤ 3, and a child order is
  dropped when its parent is met or removed); job priorities touch the hottest code (job
  search): keep the sort cheap (priorities are rare; default path unchanged when the map is
  empty). Do we want masters to also *block* others (DF's profile)? The existing workshop profile
  already does; masters only teach.
- **Parallel work:**
  - **Writes:** supply.rs, ledger.rs (new), orders.rs (most), game.rs `find_craft` and job search
    (ECO-20), psyche.rs `issue_mandate`, buildings.rs (Building fields, Order.prio),
    overseer.rs `ov_orders`, troubles.rs `boost`/`fix`, ui.rs orders/query/status screens,
    app.rs orders keys.
  - **Reads / depends on:** W0-B (`ov_m2`, Trial stats); O10's `follow_vein(p, filter)` for
    mining requests (stub: falls back to `ov_mine`); O03-c `Item.maker` (soft).
  - **Hot spots:** game.rs `find_craft` (O03-c's maker hook is in the craft *completion*, a
    different function — fine); the job search (only O08 touches it in M2); overseer.rs
    `ov_orders` (only O08 and O03-a's furniture keep lines: put O03's lines in
    `ambition_orders`-style helper `m2_orders(&o, &mut orders)` in ov_m2.rs).
  - **Agent size:** O08-a (steps 1-4, 3 sessions), O08-b (5-7, 3 sessions); O08-b's step 6 is the
    riskiest and can be deferred without blocking the milestone.

### O09 Legends worth telling  (effort: 4 sessions; risk: medium)

- **Design:**
  - *Named episodes and battles (WLD-01).* `SiegeNote` gains `#[serde(default)] name:
    Option<String>, broke_at: Option<Pos>, kill_spots: Vec<Pos>` (positions of foes slain and
    dwarves lost, capped 64). At the siege's end (`annals_tick`) `name_episode(i)` in new
    `game/episodes.rs`: the place = `place_at(mode of kill_spots)` (O03's places: "Copper Gate",
    "the Hall of …") or a coined place; the kind from outcome and numbers: routed with leader
    slain "the Rout at X", withdrew "the Siege of X", ≥ 3 dwarves lost "the Massacre at X",
    the fortress fell "the Fall of <fort>", a bridge pit kill (O04) "the Drowning at X". Other
    episodes: a megabeast fight ("the Wyrm's Fall"), three or more deaths in a day without a
    siege ("the Black Day of <month>"), an expedition's great hunt. `Game.episodes: Vec<Episode
    { name, kind, from, to, at, dead: Vec<String>, foes: u32 }>` (W0-B adds the field). Uses:
    the legend (`legend()`), the chronicle (`YearLog.siege` gets the episode name, so titles read
    "The Year of the Rout at Copper Gate"), the batch `--story`, the game-over page, the reel's
    clip title.
  - *Old battlefields (WLD-12).* After a named battle, a `PlaceKind::Battlefield` place around
    `broke_at` (radius 4) named "the field of the Rout"; goblin bones there are left unhauled
    (refuse haul skips `forbidden` items: their corpses are forbidden on the spot) and
    stay as `Bone` items; dwarves passing (daily check like `monument_daily`) think by
    personality: honour (+ for brave: "remembered the fallen of the Rout"), shun (anxious, +
    stress); a ghost of a dwarf who died there (if unburied — today's ghosts) walks there at
    night. The overseer: sentimental raises a memorial `Slab` there (existing building), vain
    names it grandly, cautious adds traps where they broke in (SIE-01 link with S04's "learns").
  - *Ages named by the world (WLD-13).* `Overworld` gains `#[serde(default)] ages: Vec<(i32
    from, String name)>`; at each new year `name_age()` (new `overworld_ages.rs` or in
    episodes.rs) looks at the world: ≥ 3 megabeasts alive "the Age of Dragons/Wyrms"; one civ
    with ≥ half the sites "the Age of <civ>"; the fortress with ≥ 30 legends and a monument
    "the Age of <fort>". A new age is a legend and a chronicle line; legends header shows it.
  - *Legends that drift (WLD-09).* `overworld.rs: Event` gains `#[serde(default)] told:
    Option<String>, fame: u8`. At the new year, each fort event older than two years has a 20%
    chance to drift (`fn retell(&Event, hash) -> String`): numbers grow ("five goblins" → "a
    dozen"), a name is swapped for a hero's, it becomes a tale ("they say …"), or it fades (fame
    0, dropped from the chronicle and engravings). Visiting bards (`visitors.rs`) tell an
    invented tale now and then, recorded with `fame` 1 and a mark "(sung by a bard)". Engravings
    (`detail.rs: pick_scene`) and the chronicle use `told` when present; the legends viewer
    shows both ("as told now").
  - *Forgotten lore (WLD-18).* `Overworld` gains `#[serde(default)] known: Vec<bool>` aligned
    with `events` (empty = all known, so old saves are unchanged). At embark: known = events of
    our civ, of sites within 20 cells, and the last 30 years; everything else forgotten. The
    legends viewer after embark shows known events and "… (forgotten)" gaps; rediscovered by
    an expedition to a ruin (`expeditions.rs` Goal::Ruin: reveal 5-10 events about that site
    and its civ), visitors' tales (2 a visit), and engravings found in ruins (mysteries). The
    overseer values ruin expeditions more when > 50% is forgotten (Venture link via `Sense::
    Wanderlust` × 1.2 for pious/greedy). Pre-embark legends (from the world map) still show all.
- **Steps:**
  1. *Named episodes and battles* · episodes.rs, annals.rs (fields, `kill_spots` writes in
     `note_foe_slain`/`note_death`, end-of-siege call), chronicle.rs (title uses the name),
     overseer.rs `run_trial` story · test `episodes.rs: a_siege_goes_into_the_chronicle_under_a_name_and_names_where_it_broke`
     (the Done-when: `force_siege`, kill the goblins at a spot inside a fitted room, end the
     siege: the SiegeNote has a name, a Battlefield place covers the spot and carries the
     episode's name, `write_chronicle_for_test` title contains it).
  2. *Old battlefields remembered* · episodes.rs (`battlefield_daily` via m2hooks), refuse/haul
     skip, ov_m2 (`ov_memorial`) · test `episodes.rs: dwarves_passing_a_battlefield_remember_it`.
  3. *Ages named by the world* · episodes.rs or ages.rs, overworld.rs (field), family.rs
     `new_year` call · test `ages.rs: three_dragons_make_an_age_of_dragons`.
  4. *Legends that drift* · overworld.rs (Event fields), drift fn, visitors.rs (bards),
     detail.rs `pick_scene`, ui legends · test `episodes.rs: an_old_tale_grows_in_the_telling`.
  5. *Forgotten lore* · overworld.rs (`known`), embark init, expeditions.rs/visitors.rs reveal,
     ui.rs `legends_lines` · test `ui.rs: forgotten_legends_show_as_gaps_until_found`.
- **Overseer & screen:** the overseer names nothing itself except through its voice (a vain one
  coins grander names: `names::coin` with a `grand` flag); it builds memorials and traps at
  battlefields; it weighs ruin expeditions by lost lore; its journal tells the episode by name.
  Screen: chronicle titles, legends header (the age), legends viewer with gaps and retellings,
  look panel for battlefields, `--story`.
- **Measure:** every siege named in the batch (`M2Stats` and the report's siege table gets a name
  column); chronicle titles that name an episode in ≥ 50% of war years.
- **Risks / open questions:** `Event` and `Overworld` are world-save structs shared by all forts
  in a world: fields must default; `known` must stay aligned when events are pushed (push
  `true` with each new event in `legend()`). Drift changes engravings' text (fine) but not
  `events.text` (the truth stays for the report).
- **Parallel work:**
  - **Writes:** episodes.rs, ages.rs (new), annals.rs (SiegeNote fields, end hook), chronicle.rs
    (its own `lines_o09` fn), overworld.rs (`Event` fields, `ages`, `known`), trade.rs `legend`
    (push `known`), detail.rs `pick_scene`, visitors.rs, expeditions.rs (reveal), ui.rs
    `legends_lines`, ov_m2.rs (one line), m2hooks.rs (one line).
  - **Reads / depends on:** W0-B places/names/saga/episodes field; O03-b's `deed_at` and
    `place_at` names; O04's SiegeNote counters (soft).
  - **Hot spots:** annals.rs SiegeNote (O04 also adds fields: agree the full field list now —
    `trapped, fell, drowned, broke_on, name, broke_at, kill_spots` — and add them all in W0-B);
    overworld.rs `Event` (only O09).
  - **Agent size:** O09-a (steps 1-2, 2 sessions), O09-b (3-5, 2 sessions), parallel.

### O10 A miner's craft  (effort: 4 sessions; risk: medium)

- **Design:**
  - *Dig speed by pick and rock (DIG-06).* `material.rs`: `fn hardness(self) -> u32` (soil 20,
    sandstone 50, limestone 55, marble 65, granite 80, basalt 85, obsidian 95, ores 60-70) and
    `fn edge(self) -> u32` for tool metals (copper 60, bronze 75, iron 90, steel 120; none 30).
    `game.rs: Task::Dig` base = `dig_ticks` scaled by `hardness * 100 / edge` (calibrated so
    copper on granite ≈ today's 140), felling by axe edge. `work_time(base, level)` gives a
    legendary (level ~15) ≈ 4× a novice today: retune so legendary = 2× novice as the idea says
    (`base * 10 / (10 + level)` for digging only). The overseer forges steel/iron picks when the
    forge stands (`ov_orders` adds ForgePick 1 + miners/2 when metal allows).
  - *Follow the vein (DIG-11 / ECO-14).* `Game.veins: HashMap<Pos, VeinFilter>` (sparse,
    `#[serde(default)]`) marks designations that spread. In `complete_dig(q)`, if `q` was a vein
    designation: after `reveal_around`, every revealed neighbour (26-neighbourhood, so veins go up
    and down) whose `mat` passes the filter (`Ore(Some(m))` same mat, `Ore(None)` any ore,
    `Gem`) and isn't designated gets `Desig::Dig` and a `veins` entry — capped at 200 tiles per
    vein. `pub fn follow_vein(&mut self, p: Pos, f: VeinFilter)` is the API (used by O08's
    supply and the overseer). Player: in Designate mode `v` "follow vein" (ore only) and `V`
    (gems). Overseer: `ov_mine` stops scanning a 41×41×9 box daily (`ambitions.rs: ov_mine`) and
    instead calls `follow_vein` on newly revealed ore tiles kept in `Game.found: Vec<Pos>`
    (pushed in `reveal_around` when a revealed tile is ore/gem): cheaper too.
  - *Prospecting and finds (DIG-12).* In `complete_dig`, the first tile dug of a cluster (no
    dug neighbour of the same mat) announces "Urist has struck a vein of hematite!"
    (`msg_at`, kind Job) and records a saga event; gems "a cluster of garnets". Smoothing a wall
    (`detail.rs: finish_detail`) reveals the rock one tile behind it on the same level.
    Prospector's report on day 1 (`ov_setup` or for players at embark): counts of ores, flux
    stone, clay and gems within 30 tiles of the wagon and 12 levels down, as a status line and a
    note; the overseer's brain gets `Sense::Ore` (W0-B adds the enum value) = ore richness,
    feeding Delve +1.0 and Prosper +0.5.
  - *Warm and damp stone (DIG-21).* Before a miner starts a `Dig` (`game.rs: find_dig` /
    `Task::Dig` arrival), if any unrevealed tile within 2 has water ≥ 1, aquifer or magma:
    the designation is suspended (`Game.damp: HashSet<Pos>`), "Urist has found damp stone"
    (warm for magma), unless `Game.dig_wet_ok` (standing order, toggled in the orders screen).
    Overseer: cautious/steady leave it and mark the plan slot blocked (`plan_taken`), bold and
    reckless set `dig_wet_ok` for that spot ("Without a second thought, it ordered the miners on
    through the damp") — losing is fun.
  - *Surviving cave-ins (DIG-07).* `disasters.rs: cave_in`: a dwarf under falling rock is
    *buried* 60% of the time instead of only wounded: the tile becomes `Wall` of rubble
    (`Tile.mat` = the rock, `built_from` = None, a `Game.rubble: HashMap<Pos, u32 dwarf>`),
    the dwarf gets `Dwarf.buried: u64` (`#[serde(default)]`, tick buried; they don't act; thirst
    and suffocation: dead after DAY "suffocated under the rubble"), and a priority-1 dig
    designation is placed on it. Digging it frees them (wounded). A "Trapped!" spotlight with
    the name; the overseer sends extra miners (`hands_for(Mining, +2)`), a sentimental one
    notes it. Creatures can be buried too (simpler: killed).
  - *Gems in their host rock (DIG-33).* New Mats: `Garnet` (in basalt), `Onyx` (in limestone),
    `Jasper` (in sandstone), `Lapis` (in marble); Amethyst stays in granite. `is_gem` extended;
    values; `terrain.rs` gem placement picks by host (and the Whispering Stone keeps working
    for any gem). Cut gems keep their mat.
  - *Digging out boulders (DIG-34).* `TileKind::Boulder` (append) placed by `terrain.rs` on
    rocky surface (hills, mountains, badlands; ~0.5% of open surface tiles), drawn `●` in rock
    colour, not walkable (like a tree). `Desig::Dig` is valid on it; digging gives a Boulder item
    and leaves floor. The overseer's `fix(Stone)` quarries embedded boulders near the wagon
    first ("set miners to break out the boulders by the wagon").
- **Steps:**
  1. *Dig speed by pick and rock* · material.rs, game.rs `Task::Dig`/chop, overseer.rs
     `ov_orders` (picks) · test `jobs.rs: a_steel_pick_digs_granite_faster_than_copper` and
     `one_year_timing` re-checked.
  2. *Follow the vein* · game.rs `complete_dig` (spread), new veins.rs (`follow_vein`,
     filters), world.rs `reveal_around` (push found), ambitions.rs `ov_mine` (rewrite), app.rs
     designate keys · tests `veins.rs: a_vein_designation_spreads_along_the_ore` and
     `veins.rs: gems_only_follows_only_gems`.
  3. *Prospecting and finds* · veins.rs (announce), detail.rs (reveal behind), ov_setup
     (report), ambitions.rs (`Sense::Ore` links) · test `veins.rs: striking_a_vein_is_announced_once`.
  4. *Warm and damp stone* · game.rs dig start, overseer choices · test `jobs.rs:
     miners_stop_at_damp_stone_unless_told`.
  5. *Surviving cave-ins* · disasters.rs, dwarf.rs (`buried`), game.rs (`update_dwarf` skip,
     death), overseer rescue · test `disasters.rs: a_buried_dwarf_is_dug_out_alive`.
  6. *Gems in their host rock, and boulders in the ground* · material.rs, terrain.rs,
     world.rs (TileKind), glyphs.rs · tests `terrain.rs: each_gem_lies_in_its_own_rock`,
     `terrain.rs: boulders_can_be_quarried`.
  7. *Done when* · test `veins.rs: a_delver_finds_more_ore_than_a_builder` (seed 5, two
     seasons each, Delver vs Builder: `ledger.mined` ore (O08's ledger, or a `Game.ore_mined`
     counter if O08-b hasn't landed) Delver ≥ 1.5× Builder, and a Delver note says it followed a
     vein).
- **Overseer & screen:** the Delver gets a craft: prospect drifts (`ov_shaft` digs short drifts
  off each shaft level in four directions and follows any vein met), follows veins, sends extra
  miners after finds; notes "Followed a vein of malachite down three levels". Screen: find
  announcements, the prospector's report (status), damp/warm warnings, rubble rescue card, new
  gem names, boulders on the surface.
- **Measure:** `M2Stats.ore_mined` by style (Delver ≥ 1.5× Builder); deaths "crushed" fall by
  half; the batch's year timing (`one_year_timing`) not slower (veins replace the box scan).
- **Risks / open questions:** dig-speed changes slow early digging (copper picks on granite):
  calibrate so the median fort's stair+gatehouse is done by the same day as now. Damp-stone
  stops could stall plans near rivers (the plan already avoids known water). New Mat values must
  be added to every exhaustive `match` over Mat (compiler finds them).
- **Parallel work:**
  - **Writes:** material.rs, terrain.rs, veins.rs (new), game.rs (`Task::Dig`, `complete_dig`,
    dig start, `update_dwarf` skip for buried), world.rs (`reveal_around` push, TileKind),
    disasters.rs, dwarf.rs (`buried`), ambitions.rs (`ov_mine`, `ov_shaft`, Sense::Ore links),
    detail.rs (one line), overseer.rs (`ov_orders` pick line).
  - **Reads / depends on:** W0-B (Sense::Ore placeholder, LogEntry `msg_at`, saga); O08's ledger
    (soft).
  - **Hot spots:** game.rs `Task::Dig` arm and `complete_dig` (only O10 in M2; O03-c touches
    `desig_valid` for built walls: a different function), dwarf.rs `Dwarf` (+1 field: add in
    W0-B), Mat/TileKind enums (append).
  - **Agent size:** O10-a (steps 1-3 + 7, 2 sessions), O10-b (4-6, 2 sessions), parallel after
    step 2 is merged (O10-b's buried dwarves use step 2's priority-free dig).

### O11 A builder's hands  (effort: 4 sessions; risk: medium)

- **Design:**
  - *One-column stairs (ENG-09).* `game.rs: designate(a, b, Desig::StairUD)` over a box spanning
    z: per column, the lowest z gets `StairUp`, the highest `StairDown`, between `StairUD` (today
    every tile gets UD, leaving a stair down into rock at the bottom). Over open air (`Open`
    tiles), a StairUD designation plans a constructed StairUD instead (`plan_constructions`).
    The overseer's stair (`ov_setup`) and inner stair use it.
  - *Builders don't wall themselves in (ENG-10).* `construct.rs: find_build_tile` picks a stand
    tile that is not itself a pending construction and that still reaches a tile outside the
    pending set's bounding box (one BFS limited to 200 nodes, cached per construction batch by
    `dig_version`); digging picks the nearest designated tile first (it does; add a test).
  - *Bulk rooms and overlapping zones (ENG-17).* Overlap: keep `Tile.zone` as the primary zone and
    add `Game.zone_over: HashMap<Pos, Vec<u32>>` for extra zones on a tile (only compatible
    kinds overlap: Dining/Meeting/Tavern/Temple with each other; stockpiles and farms never).
    New `Game::zones_at(p) -> impl Iterator<Item = &Zone>`; consumers switched: `rooms.rs:
    in_dining_hall`, meeting-area search, tavern/temple break searches, `zone_kind_at` (returns
    the primary; new `has_zone_kind(p, k)`). `add_zone` over an existing compatible zone adds to
    the overlay instead of refusing. Bulk: in Zone mode `b` toggles "each enclosed room its own
    zone": the box is split with `psyche.rs: room_at` flood fills into one zone per enclosed
    area. The overseer's `ov_fit` Dining also makes it the Meeting area while there's no Meeting
    room (an early-fort win), and Bedrooms rows get one Bedroom zone per cell (bulk) so each
    dwarf owns a room.
  - *Named halls (ENG-16).* `Zone.name`, `Building.name: Option<String>` (`#[serde(default)]`);
    `N` on a zone or building in look/query asks for a name (text field as in O01's rename) and
    registers a place (`name_place(.., by_player: true)`, never renamed by deeds). Shown in look,
    hover and map labels when zoomed.
  - *Build over built floors (ENG-45).* `construct.rs: can_construct` accepts a tile whose
    construction is a Floor for Wall/Fortification/stairs/Ramp; `do_build_tile` removes the old
    floor first and drops its material.
  - *Better levers (ENG-33).* `Building.pending_links: Vec<u32>` and `timer: Option<(u64 every,
    u64 next)>` (`#[serde(default)]`). New `Game::request_link(lever, target) -> Result<String,
    String>` claims a mechanism and queues a job: a dwarf with Mechanics fetches it to the target,
    works 100 ticks, then the link is made (`links.push`). `link_lever` stays as the instant
    form for tests and scripts. Lever query lists links with `u` unlink (the mechanism is
    returned) and `t` set a timer (pull every N days). `Mode::LinkLever` uses `request_link`.
    The overseer's gate (`gatehouse.rs`) switches to `request_link` and marks "linked" only when
    `links` holds the bridges (it already checks that).
  - *Roads and wagon access (ENG-35).* `ConstructKind::Road` (a paved floor; walkers move 20%
    faster on it via `path` cost: road tiles cost 4 instead of 5). Wagons: on caravan arrival
    (`trade.rs`), `wagon_path(depot) -> Option<Vec<Pos>>` = an A* with `Mover::Wagon` (a 3×3
    clearance check per step) from the map edge to the depot, cached by `dig_version`; no path:
    "The merchants could not bring their wagons: no wide road to the depot" and goods ×0.5 (pack
    animals only). Depot query shows the wagon path or where it's blocked (overlay). The overseer
    chops trees along a 3-wide line from the depot to the nearest edge and, if Builder or greedy,
    paves it (`ov_m2 → ov_road`); names it as a Road place ("the Copper Road").
- **Steps:**
  1. *One-column stairs* · game.rs `designate`, construct.rs · test `game.rs:
     a_stair_column_has_up_at_the_bottom_and_down_at_the_top`.
  2. *Builders don't wall themselves in* · construct.rs `find_build_tile` · test `construct.rs:
     a_builder_walls_a_ring_and_ends_outside_it`.
  3. *Bulk rooms and overlapping zones* · game.rs/rooms.rs (`zone_over`, `zones_at`,
     `add_zone`), consumers, app.rs zone `b`, overseer `ov_fit` (Dining as Meeting, bedroom
     zones) · test `rooms.rs: a_dining_hall_that_is_also_the_meeting_area_works_for_both` (the
     Done-when part 1: zone Dining and Meeting on the same tiles; a hungry dwarf eats at its
     table and idle dwarves gather there).
  4. *Named halls* · zone/building fields, app_m2.rs (rename), places · test `ui.rs:
     name_a_hall_and_see_it_on_look`.
  5. *Build over built floors* · construct.rs · test `construct.rs: a_wall_goes_up_on_a_built_floor`.
  6. *Better levers* · traps.rs (`request_link`, unlink, timers, job), game.rs job dispatch
     (one arm), gatehouse.rs (request), app.rs LinkLever · test `traps.rs:
     linking_a_lever_sends_a_mechanic` (the Done-when part 2: request a link; links empty, a
     mechanic walks to the bridge; after the job, linked and one mechanism used) and the S04
     gate tests still passing.
  7. *Roads and wagon access* · construct.rs (Road), path.rs (cost, `Mover::Wagon`), trade.rs
     (arrival check), ov_m2 (`ov_road`), ui depot overlay · test `trade.rs:
     wagons_need_a_wide_road_to_the_depot`.
- **Overseer & screen:** the overseer uses one-column stairs, overlapping dining/meeting, bedroom
  zones, lever jobs, roads; notes "Laid the Copper Road to the depot, wide enough for wagons".
  Screen: zone overlays (look lists every zone on a tile), names, lever query, wagon path.
- **Measure:** batch: trade value per caravan (report), early meeting area by day 30 in ≥ 90%
  of forts; the bridge-linked day must not slip by more than 2 days on average now that a
  mechanic walks over (S04's bridge-up share must hold).
- **Risks / open questions:** overlapping zones touch the room/meeting searches (hot-ish paths):
  the overlay map is consulted only when `t.zone` alone doesn't answer. Lever jobs delay the
  first bridge link (risk to M1's siege numbers): keep the overseer's first gate link instant
  (it's "the expedition's mechanic") as an option if the batch suffers — a decision for the user.
- **Parallel work:**
  - **Writes:** construct.rs, rooms.rs, game.rs (`designate`, `add_zone`, job dispatch arm),
    traps.rs (links), path.rs (road cost, Mover::Wagon), trade.rs (arrival), buildings.rs and
    game.rs Zone (name fields), gatehouse.rs (request_link), ov_m2.rs, app.rs/app_m2.rs, ui.rs
    look/query.
  - **Reads / depends on:** W0-B places; O04-a (gatehouse.rs is also edited by O04: serialize
    O11 step 6 after O04-a, or limit O11's gatehouse edit to swapping one call).
  - **Hot spots:** path.rs (cost change is global: coordinate with any path perf work; measure
    `one_year_timing`); game.rs `add_zone`; Zone/Building structs (+1 field each, add in W0-B).
  - **Agent size:** O11-a (steps 1-5, 2 sessions), O11-b (6-7, 2 sessions).

### O12 Inspired engineers  (effort: 3.5 sessions; risk: medium)

- **Design:**
  - *Blueprints (CRF-03).* New `game/inspiration.rs`: `Inspiration { dwarf: u32, work: GrandWork,
    since: u64, drawn: Option<u32 item>, adopted: bool, done: bool }` in
    `Game.inspirations: Vec<Inspiration>` (W0-B field). `GrandWork { Hall(Shape), Rotunda,
    Gallery, Temple, KillingHall, PitBridge, Road, Automaton }`. Sources (seasonal, in
    `m2_daily` at season start): a dwarf with Masonry/Engraving/Mechanics skill ≥ 8 and artistry
    > 20 has 25%; anyone who made an artifact; a dwarf who admired the monument this season (10%).
    The inspired dwarf goes to an Office zone or any table (`Task::Draw`, 600 ticks) and makes
    `ItemKind::Blueprint` (label "plans for the Pillared Hall of Kib Ironbrow"). The overseer
    sees it: `Sense::Inspired` (W0-B enum value) fires 1.0 while an unadopted blueprint exists
    and links Architect +1.5 (+0.5 vain, +0.3 pious, −0.8 greedy, −0.6 stubborn) and, for
    defence works, Fortify +1.2. Adopting: `begin_ambition(Architect)` with `o.adopted =
    Some(blueprint id)`; `ambitions.rs: pursue` Architect pushes `wants: Grand` with the
    blueprint's shape (`layout.rs: pick_shape` reads `o.adopted`), O04's gatehouse stage for
    KillingHall/PitBridge, O11's road for Road. Finished: `Room.name`/place "the Pillared Hall
    of Kib Ironbrow" (`name_place`, weight 8), legend "… drawn by Kib Ironbrow, was finished",
    the dwarf earns "the Architect" (`earn_epithet`), a thought, a spotlight "A dream in stone"
    and a reel clip. Ignored for a year: the dwarf sulks (stress +20 "was ignored by the
    overseer"), faith −5 (O07).
  - *Automatons (CRF-28).* `Species::Automaton` (creature.rs; tame, no blood: `body` doesn't
    bleed, doesn't eat or sleep). Made two ways: a mechanic's fey mood (`psyche.rs: start_mood`
    allows `Labor::Mechanics`; its artifact list gains "automaton" for mechanics) or an Automaton
    blueprint built at the Mechanic's workshop (Recipe::Automaton: 10 mechanisms + 4 bars,
    600 ticks). It guards: stands at a post (`Creature.post: Option<Pos>`) and attacks hostiles
    within 6 tiles, returning after; repaired (hp restored) by a mechanic with a mechanism when
    below half. Named ("the Brass Warden of Kib", `names::coin(Automaton)`). The overseer posts
    it at the gate (`gatehouse::stations`) — Warlord/cautious adopt automaton blueprints first.
  - *Luxury trinkets (CRF-35).* `ItemKind::Trinket` with `label` kind: mirror (polished
    obsidian or a metal bar, Craftsdwarf), clock (2 mechanisms + bar, Mechanic), mask (leather
    or bone), dice (bone), cards (cloth). Values above crafts; liked trinkets give a happy thought
    when seen in the owner's room; a tavern with dice or cards (items in the tavern zone) adds a
    carouse bonus (`needs.rs` carouse satisfaction ×1.3). The overseer orders them under Prosper
    (trade) and puts dice/cards in taverns under Revel.
- **Steps:**
  1. *Blueprints become ambitions and named works* · inspiration.rs, items.rs (Blueprint),
     game.rs task arm (Draw), ambitions.rs (`Sense::Inspired` links, Architect pursue),
     layout.rs `pick_shape`, naming via places · test `inspiration.rs:
     a_dwarfs_inspiration_becomes_an_ambition_and_a_work_bearing_their_name` (the Done-when:
     dug-in Builder, make an inspiration for dwarf X with `GrandWork::Hall(Pillared)`, run until
     drawn and adopted (force `begin_ambition` if the odds roll is slow, but assert the sense
     links Architect ≥ +1.5), then until the Grand room is fitted (≤ 2 seasons): a place named
     with X's name exists, X has the epithet).
  2. *Automatons* · creature.rs (species, post, guard AI), buildings.rs (recipe), psyche.rs
     (mood), gatehouse.rs (post) · tests `creature.rs: an_automaton_guards_its_post_and_does_not_bleed`,
     `psyche.rs: a_mechanic_in_a_mood_can_make_an_automaton`.
  3. *Luxury trinkets* · items.rs, buildings.rs (recipes), needs.rs (carouse bonus),
     ambitions.rs (`ambition_orders` Prosper/Revel) · test `needs.rs: dice_in_the_tavern_cheer_a_carouse`.
- **Overseer & screen:** the overseer gets ambitions it didn't plan (the sense in the brain, with
  the dwarf's name in the side panel: "Kib's plans for a pillared hall"); notes when it adopts or
  sets plans aside; works carry the dreamer's name on look, in the chronicle and legends.
- **Measure:** `M2Stats.inspirations` (drawn, adopted, finished); target: ≥ 1 adopted work in
  half the 3-year forts; automatons' kills in the siege table.
- **Risks / open questions:** automatons could trivialise early sieges (cap one per fort per
  year; 10 mechanisms is a lot early). GrandWork kinds that need O04/O11 fall back to a Hall
  when those items haven't landed.
- **Parallel work:**
  - **Writes:** inspiration.rs (new), creature.rs (species, post), psyche.rs (`start_mood`,
    artifact list), items.rs, buildings.rs (append), needs.rs, ambitions.rs (links, pursue,
    `ambition_orders`), layout.rs (`pick_shape`), game.rs (task arm), gatehouse.rs (post).
  - **Reads / depends on:** W0-B (Sense::Inspired, inspirations field, places, names); O02
    (Grand on the plan), O03-b (`name_place` use), O07 (faith, soft), O04/O11 (soft).
  - **Hot spots:** ambitions.rs `pursue` Architect arm and `brain` links (O07 also adds links:
    keep each item's links in its own helper `links_o07(..)`, `links_o12(..)` called from
    `brain`); creature.rs species enum (append).
  - **Agent size:** O12-a (step 1, 2 sessions), O12-b (steps 2-3, 1.5 sessions).

---

### Status after wave 0 (2026-10)

The plan above predates a lot of work. Wave 0 (W0-A as part of O02, and W0-B on branch
`wave/m2-w0`) is done; this section is what a wave-1 agent codes against. **Where it and the
item sections disagree, this section wins.** Every name below exists on master after wave 0.

**Already done before wave 0, so packages skip these steps**

- O01: name, two traits, temper sense, voiced notes, re-roll (`personality.rs`). Open: pick
  traits by hand, note tags (`NoteKind`), intent line, commands on cards.
- O02 (P2): `Facing` (`layout.rs`, with `Facing::at(start, fwd, side)`, not `Overseer::rel`),
  per-level plans in `Overseer.levels: Vec<blueprint::LevelPlan>` (each with its own
  `facing`), the `P` ghost overlay. The gatehouse itself still runs east (`base.offset`).
- O03: smoothing and engraving, interiors, suites, room titles (`architect.rs`:
  `o.interior.places`, rebuilt daily by `ov_places`; `Game::place_name(p)`), wealth tier is
  `o.interior.tier` (no `Overseer.tier`). Statues of legends (`depict.rs`).
- O04: towers, curtain wall, walkway (`towers.rs`, `fortify.rs`).
- O05: mood strip (`ui/moodstrip.rs`, `Target::Dwarf`), director camera (`src/director.rs`,
  `App.director`), highlight reel and replay (`reel.rs`, `ui/highlights.rs`, `Mode::Highlights`),
  ambient particles, the news ticker. **P5 is down to step 3, sound-based reports.**
- O06: life story and portrait (`lifestory.rs`, `ui/life.rs`, `Mode::Life`), eulogies, death
  scenes in the chronicle (`Game::death_scenes`).
- O09: named battles and episodes (`stories.rs`: `Game.stories.episodes`), places named for a
  deed (WLD-02: `Game.stories.places`), sagas told on cards (`sagas.rs`, `Game.sagas`).
- Shared machinery from M1: the schedule (`schedule.rs`), finder table (`finders.rs`),
  overseer hooks (`overseer_hooks.rs`), knobs and counters (`knobs.rs`: `Game::knob`,
  `Game::count`), the square-tile window (`src/window/`).

**Renamed or replaced from the plan text above**

| Plan says | Use instead | Why |
|---|---|---|
| `saga.rs`, `Game.saga`, `SagaKind`, `SagaEvent`, `record`/`saga_of`/`saga_since` | `memory.rs`: `Game.memory: Memories`, `MemoryKind`, `Memory`, `Game::record(kind, who, at, text, weight) -> u32`, `Game::memory_of(id)`, `Game::memory_since(tick)` | `sagas.rs` already has `SagaKind` and `Game.sagas` (story arcs) |
| `places.rs` with its own `Place` store, `Game.places` | `places.rs`: `PlaceKind`, `Game::place_at(p) -> Option<&stories::Place>`, `Game::name_place(kind, a, b, name, why, weight, by_player) -> usize` over `Game.stories.places` | the WLD-02 places already live there; `stories::Place` gained `kind`, `b` (a box), `weight`, `by_player` |
| `names.rs` `coin`, `hash` | as planned: `names::coin(seed, NameTheme)`, `names::hash(&[u64])` (on `stories::hash`) | |
| `m2hooks.rs` `m2_daily`/`m2_tick`, `ov_m2.rs` `ov_m2` | a line in `schedule.rs` (in the group for its cadence) and in `overseer_hooks.rs` (`ov_daily_hooks`, `ov_alert_hooks`, `on_siege_end`) | those registries already exist and serve |
| `links_<item>` helpers in `ambitions.rs: brain` | a row in `overseer_hooks.rs: extra_senses` (sense, level 0..1, per-ambition weights) | `brain` already reads extra senses; no `brain` edit needed |
| `Trial.m2: M2Stats`, `m2stats.rs` | `game/m2.rs: Game::m2_stats()` (called once at the end of `run_trial`) writing `m2.*` counters; `report.rs: overseers(trials)` prints them by style | batch numbers go through counters (PLAN.md), and the batch file stays byte-identical while they are empty |
| `src/glyphs.rs` | `src/ui/looks.rs` (`tile_look`, `foliage`, `water_glyph`, still `ratatui::Color`; `crate::ui::tile_look` still works) | `src/window/glyphs.rs` is the font cache; the reel already uses `ui::tile_look` |
| `src/app_m2.rs`, `src/ui_overseer.rs`, `ui_units.rs`, `ui_reel.rs` | `src/app/m2.rs`, `src/ui/overseer.rs` (new files in the existing `app/` and `ui/` folders); P7 creates `src/ui/units.rs` | |
| `Overseer.tier` | `o.interior.tier` | exists |
| `Mode::Reel { clip, frame }` | `Mode::Highlights { sel }` | exists |
| `Game.reel`, `Game.episodes` | `Game.reel`, `Game.stories.episodes` | exist |

**Wave 0 added (all saved fields `#[serde(default)]`; the 6-fort batch is byte-identical)**

- Stores: `Game.memory` (with first writers: deaths, births, weddings, epithets, artifacts,
  madness, appointments, migrants, sieges and their ends, the monument), `Game.veins:
  veins::Veins { follow, found }` and `veins::VeinFilter`, `Game.ledger: supply::Ledger`,
  `Game.inspirations: inspire::Inspirations`, `Game.ov_fell: Option<(u32, String)>`,
  `Game.zone_over` (not saved; rebuild from `zones`).
- `LogEntry { at: Option<Pos>, kind: LogKind }`; `logkind.rs`: `LogKind` (13 kinds,
  `ALL`, `name`, `classify(text, color)`), `LogEntry::kind()` (the set kind, else classified),
  `Game::msg_at(text, color, at, kind)`. `Game::msg` is unchanged.
- Shared fields: `Dwarf.buried`; `Zone.name`; `Building.{name, master, min_skill,
  pending_links, timer}`; `Item.{kills, maker, decor}`; `DeathNote.last_words`;
  `SiegeNote.{trapped, fell, drowned, broke_on, name, broke_at, kill_spots}`;
  `YearLog.{named, reigns}`; `Spotlight.actions: Vec<Command>`. Fields on anything the batch
  file holds (`DeathNote`, `SiegeNote`) are `skip_serializing_if` empty: keep it that way.
- `Overseer.{dwarf, faith (60 by default), faith_log, reigns, journal, gate_stage,
  orders_seen, seen_deaths, seen_sieges, commands, adopted}`; types in `reign.rs` (`Reign`,
  `ReignEnd`, `JournalPage`, `sixty()`) and `commands.rs` (`Command { RaiseBridge, Shelter,
  StandDown, Muster, Recall }` with `key`/`label`/`ALL`; `Game::command_overseer(c)` only
  records the command so far).
- `Sense::{Doubt, Ore, Inspired}`, in `Sense::ALL` and the brain, silent until their package
  gives them a level and links through `extra_senses`.
- `Mode::{OverseerTraits { sel }, Journal { year }, Family { id, sel }, Announce { sel }}`:
  "coming soon" placeholders; keys in `app/m2.rs` (`key_overseer_traits`, `key_journal`,
  `key_family`, `key_announce`), drawing in `ui/overseer.rs` (`draw_overseer_traits`,
  `draw_journal`, `draw_family`, `draw_announce`). `App.{announce: app::m2::Announce,
  step_pending}`.
- Hook points, one line each: `App::key_m2(k) -> bool` (asked first by `key_normal`),
  `ui/overlays.rs: draw(f, app, over)` (end of `ui::draw` in Normal mode),
  `chronicle.rs: lines_m2(&log)` (after `lines_basic`, before the dead), `m2.rs:
  m2_stats()`.
- File splits (verbatim moves): `gatehouse.rs` (`ov_gate_alert` = the door-barring and
  bridge half of `ov_alert`, `gate_lever`, `gate_bridges`, `ov_defences`,
  `gate_wants_stone`, `gate_parts_missing`, `gatehouse`), `fit.rs` (`fit_bedrooms`,
  `fit_workshops`, `fit_dining`, `fit_dormitory`, `fit_tavern`, `fit_hospital`, `fit_tomb`,
  `fit_barracks`, `fit_grand`; `ov_fit` keeps the one-line arms), `chronicle.rs:
  lines_basic`, `ui/looks.rs`.
- Not added (the one package that needs it adds it): `ItemKind::Trophy`,
  `BuildingKind::Display`, `Recipe::Encrust` (P4: enums with `ALL` lists the overseer and
  menus iterate, so their arms come with the feature); `WorkOrder` fields and `Cond` (P9);
  `Mode::Unit.tab`, `Mode::Units.{cat, sort}` (P7); `NoteKind` and `note_ov`'s kind (P1);
  `naming.rs: deed_at` (P3); `gatehouse.rs: stations`, `next_part` (P1/P14).

**Wave 1: what each package uses and owns**

| Package | Uses | Owns (new or main files) | Shared lines it adds |
|---|---|---|---|
| P1 O01 rest | `Mode::OverseerTraits`, `Command`, `command_overseer`, `Spotlight.actions`, `Overseer.{commands, seen_deaths, seen_sieges}`, `names::hash` | `personality.rs`, `commands.rs`, `app/m2.rs: key_overseer_traits`, `ui/overseer.rs: draw_overseer_traits`, `spotlight.rs` | `note_ov` tags (merge first); a reflect line in `ov_daily_hooks`; Autoplay `t` |
| P3 O03-b names | `name_place`, `place_at`, `PlaceKind`, `names::coin(_, Hall)`, `record(MemoryKind::Named, ..)`, `YearLog.named` | `naming.rs` (`deed_at`, `DeedKind`) | one `deed_at` line per deed site; `lines_named` in `lines_m2`. The look panel already shows `place_at` (via `story_place_at`). Room titles (`architect.rs: place_name`) don't yet look at `place_at`: if P3 makes them, note `place_name` also feeds battle names in `stories.rs`, so it shifts names, not the rng. |
| P4 O03-c crafts | `Item.{kills, maker, decor}`, `names::coin(_, Weapon)`, `record` | `crafts.rs`, `items.rs` (adds `Trophy`), `buildings.rs` (adds `Display`, `Encrust`), `heroes.rs`, `detail.rs` | maker set at craft completion in `game.rs` |
| P5 O05-a | step 3 only: `LogEntry.{at, kind}`, `msg_at`, `place_at`, `ui/overlays.rs` | `game/sounds.rs` exists (blows for the audio): put the reports in a new `sound_reports.rs` | `msg_at` swaps in combat.rs, disasters.rs, fire.rs |
| P6 O05-b | `DeathNote.last_words`, `Mode::Announce`, `App.{announce, step_pending}`, `AnnounceRule`, `LogKind`, `key_m2` (step keys `s`/`S`/`D` while paused) | `lastwords.rs`, `app/m2.rs: key_announce` + `key_m2` arms, `ui/overseer.rs: draw_announce` | one line in `kill`; app `update` honours the rules |
| P7 O06-a sheets | `memory_of(id)`, `record` for squad, wounds, recovery, duels, feats | `ui/units.rs` (new), `Mode::Unit`/`Mode::Units` (adds `tab`, `cat`, `sort`) | writer lines in military.rs, drama.rs, festivals.rs, psyche.rs |
| P8 O07-a reign + faith | `reign.rs` types, `Overseer.{dwarf, faith, faith_log, reigns}`, `Game.ov_fell`, `Sense::Doubt` (`extra_senses` row), `YearLog.reigns`, `MemoryKind::Overseer` | `reign.rs` | `ov_reign` in `ov_daily_hooks`; one line in `kill`; `lines_reigns` in `lines_m2`; `m2.reigns` in `m2_stats` |
| P9 O08-a | `supply.rs` (add `sources`, `have`, `feed`, `Source`, `MatFilter`, `Want`), `veins::VeinFilter` | `supply.rs`, `orders.rs` (adds `WorkOrder` fields with `serde(default)`, `Cond`) | `find_craft` in game.rs; `ov_orders` |
| P10 O10-a | `Game.veins`, `VeinFilter`, `Sense::Ore` (`extra_senses` row), `msg_at(.., LogKind::Job)`, `record` | `veins.rs` (`follow_vein`), `material.rs` (`hardness`, `edge`) | `Task::Dig`/`complete_dig` in game.rs; `reveal_around`; `m2.ore_mined` in `m2_stats` |
| P11 O11-a | `Zone.name`, `Building.name`, `Game.zone_over`, `name_place(.., by_player: true)`, `fit.rs` | `construct.rs`, `rooms.rs`, `fit.rs: fit_dining, fit_bedrooms` | `designate`/`add_zone` in game.rs; rename key in `app/m2.rs` |

Conflict rules from the table below still hold (P1's note tags first; P5 owns the top-row
split in `ui::draw`, P7 owns `Mode::Unit`; P9 and P10 edit different game.rs functions).

### Work packages and waves

Sessions are agent sessions; "∥" means can run at the same time.

**Wave 0 (foundations)** — W0-A facing refactor (1) ∥ W0-B scaffolding (1). They touch
different files except overseer.rs: W0-A edits function bodies, W0-B adds struct fields and the
gatehouse move — do **W0-B's gatehouse move after W0-A** (serialize those two commits: W0-A
first, then W0-B rebases; or have W0-B move the functions and W0-A apply `rel` there). Batch by
the coordinator: the byte-identical check.

**Wave 1 (independent, disjoint files)**

| Package | Items/steps | Sessions | Main files |
|---|---|---|---|
| P1 O01 | all | 1.5 | personality.rs, app_m2.rs, ui_overseer.rs, spotlight.rs |
| P2 O02-a | steps 2-4 | 2 | blueprint.rs, layout.rs, overseer.rs `ov_setup` |
| P3 O03-b names | step 4 | 1 | naming.rs, one-line hooks |
| P4 O03-c crafts | step 5 | 1.5 | crafts.rs, items.rs, heroes.rs, detail.rs |
| P5 O05-a strip/director/sounds | 1-3 | 2 | director.rs, drama_spots.rs, ui_units.rs |
| P6 O05-b last words/announce/step | 4-6 | 1.5 | lastwords.rs, app_m2.rs |
| P7 O06-a sheets | 1-3 | 1.5 | ui_units.rs (sheet tabs), app.rs Unit |
| P8 O07-a reign + faith | 1-2 | 2 | reign.rs, ambitions.rs (links helper) |
| P9 O08-a supply + conditions | 1-4 | 3 | supply.rs, orders.rs, game.rs `find_craft` |
| P10 O10-a pick/vein/finds | 1-3,7 | 2 | material.rs, veins.rs, game.rs dig |
| P11 O11-a stairs/zones/names/floors | 1-5 | 2 | construct.rs, rooms.rs, game.rs `designate`/`add_zone` |

Conflicts to manage in wave 1: P1 tags every `note_ov` call (overseer.rs, ambitions.rs,
troubles.rs, layout.rs): merge P1 step 2 **first** in the wave (it's small) and have P2/P8/P9/
P10 rebase. P5 and P7 both edit app.rs (keys, Mode::Unit) and ui.rs (`draw`): P5 owns the
`draw` row split; P7 owns `Mode::Unit`. P9 and P10 both touch game.rs (different functions:
`find_craft` vs `Task::Dig`/`complete_dig`). P4, P11 append to `BuildingKind`/`ItemKind`
(resolve by keeping both).

**Wave 2 (needs wave 1)**

| Package | Needs | Sessions |
|---|---|---|
| P12 O02-b levels + ghost (steps 5-6) | P2 | 1.5 |
| P13 O03-a finish/furniture/statues (steps 1-3, 6) | P2, P3 | 2 |
| P14 O04-a trap hall + triggers (1a, 2) | W0, M1 S04 | 1.5 |
| P15 O05-c reel + export (7-8) | W0-B glyphs | 1.5 |
| P16 O06-b tree/deaths/readouts (4-6) | P7 (and P6 soft) | 1.5 |
| P17 O07-b drift/spin/journal (3-5) | P8 | 2 |
| P18 O08-b masters/priorities/ledger (5-7) | P9 | 3 |
| P19 O09-a episodes + battlefields (1-2) | P3 | 2 |
| P20 O10-b damp/cave-ins/gems/boulders (4-6) | P10 | 2 |

Parallel in wave 2: all except P12 ∥ P13 (both blueprint.rs: P12 `pieces`/ghost, P13
`furn`/`features` — run P12 first or split blueprint.rs edits by function and accept a small
merge). P18's job-priority step (6) is the only edit to the job search: run it alone at the end
of the wave if it conflicts with anything else in game.rs.

**Wave 3**

| Package | Needs | Sessions |
|---|---|---|
| P21 O04-b pit bridge, pond, falling (1b) | P14 | 1.5 |
| P22 O04-c towers + legends (1c, 3) | P21, M1 S07, P19 soft | 1 |
| P23 O09-b ages, drift, lore (3-5) | P19 | 2 |
| P24 O11-b levers + roads (6-7) | P14 (gatehouse.rs) | 2 |
| P25 O12-a blueprints → ambitions | P12, P13, P17 soft | 2 |
| P26 O12-b automatons + trinkets | P14 | 1.5 |

P21 ∥ P23 ∥ P25 ∥ P26; P22 after P21; P24 edits gatehouse.rs (one call) and path.rs: serialize
P24's path.rs change with P21's `walkable` change (same file, different functions: fine, but
the coordinator should run `one_year_timing` after both).

Batches (coordinator, once per wave, ~40 min each): after wave 0 (identity), after wave 1
(O02 facing changes the rng: new baseline; check survival), after wave 2 (O04-a, O03-a wealth
effects, O07 faith), after wave 3 (the release batch and the `--traits` A/B runs).

### Interface contracts (build first, then code against them)

All in W0-B unless noted; signatures as written in the items above.

- `layout.rs`: `Facing`, `Overseer::{rel, rel_top, local}` (W0-A).
- `blueprint.rs` (O02-a, needed by O03-a, O12): `LevelRole`, `LevelPlan`, `plan(spec, start) ->
  Blueprint`, `Blueprint::transform`, `Slot { …, furn: Vec<Pos> }` (field added empty by O02-a so
  O03-a only fills it), `Blueprint.features: Vec<(Pos, Feature)>` (same), `plan_halls(o, level)
  -> Vec<Pos>`, `plan_ghost(z)`.
- `saga.rs`: `SagaKind`, `SagaEvent`, `Game::{record, saga_of, saga_since}`.
- `names.rs`: `NameTheme`, `coin(seed, theme)`, `hash(parts)`.
- `places.rs`: `PlaceKind`, `Place`, `Game::{place_at, name_place}`; O03-b adds `naming.rs:
  Game::deed_at(pos, DeedKind, who: &[u32])` (`DeedKind { SiegeHeld, LeaderSlain, BeastSlain,
  Massacre, Wedding, Death, Work }`) used by O04, O09, O12.
- `game.rs`: `LogEntry { at, kind }`, `LogKind`, `Game::msg_at`; `Game.{saga, places, episodes,
  inspirations, reel, veins, ledger}` fields (empty types until their package lands);
  `Dwarf.buried`, `Zone.name`, `Building.{name, master, min_skill, pending_links, timer}`,
  `Item.{kills, maker, decor}`, all `#[serde(default)]`.
- `annals.rs SiegeNote`: `trapped, fell, drowned: u32; broke_on, name: Option<String>;
  broke_at: Option<Pos>; kill_spots: Vec<Pos>`; `DeathNote.last_words: Option<String>`.
- `ambitions.rs Sense`: `Doubt, Ore, Inspired` with no links (brain shows them dark); each
  owning item adds a `links_<item>(&self, o, a, add)` helper called from `brain`.
- `overseer.rs Overseer` fields listed in W0-B §9; `Command` enum; `Game::command_overseer`.
- `gatehouse.rs`: `ov_gatehouse(o)`, `gate_lever`, `gate_bridges`, `stations(o) -> Vec<Pos>`,
  `next_part(o) -> Option<String>` (for O01's intent).
- `veins.rs` (O10-a, used by O08): `VeinFilter`, `Game::follow_vein(p, f)`.
- `supply.rs` (O08-a): `sources(input)`, `Game::{have, feed}`.
- `m2hooks.rs`: `m2_daily`, `m2_tick`; `ov_m2.rs`: `ov_m2(o)` — append one line per system.
- `M2Stats` and `Game::m2_stats()`; each package fills its own field in `m2_stats()`.
- `glyphs.rs`: `tile_look(w, p) -> (char, rgb, rgb)`, `unit_glyph(g, p) -> Option<(char, rgb)>`.

### Hot-spot discipline (do before wave 1)

1. W0-A and W0-B above (facing, scaffolding, all struct fields and enum values at once, Mode
   placeholders, step hooks).
2. Move the gate code to gatehouse.rs (W0-B §10) and the glyph functions to glyphs.rs (§5).
3. `chronicle.rs: write_chronicle`: split into `lines_basic`, and give each item its own
   `lines_<item>(&log) -> Vec<String>` (O03 named halls, O06 death scenes, O07 reigns/spin, O08
   ledger, O09 episodes) called in a fixed order: one line each in `write_chronicle`.
4. `ambitions.rs: brain`: move the per-ambition link `match` into `base_links(a, …)` and call
   `links_<item>` helpers after it.
5. `overseer.rs: ov_fit`: turn the big `match r.kind` into `fit_<kind>` functions (one per
   RoomUse) so O03 (Suite, Bedrooms extras) and O11 (Dining as Meeting, bedroom zones) edit
   different functions.
6. `app.rs`: a `fn key_m2(&mut self, k) -> bool` called first in `key_normal`, defined in
   app_m2.rs: new Normal-mode keys (`P`, `V`, `F`, `U`, `J`, `E`, `s`, `S`, `D`) go there, not
   into the big match.
7. `ui.rs: draw`: an `overlays(f, app, over)` call at the end for O02 ghost legend, O05 director
   badge and mood strip click targets.
8. `report.rs`: the `overseers` section is one function; each package adds its line there.

### Verification per package

Every package: its listed tests, `cargo test --release` printing `test result: ok`, and for any
package touching job search, pathing, digging or `walkable` (P9, P10, P11, P18, P20, P21, P24)
the `one_year_timing` ignored test within 10% of the wave's start. Saves: each package that adds
saved state runs `old_saves_load_without_m2_fields` (W0-B) after adding its fields' defaults.
Batches after merging (coordinator): wave 0 identity; P2 (rng shift: new baseline), P13 (wealth),
P14/P21/P22 (sieges), P8/P17 (faith and successions), P9 (orders), P10/P20 (digging), P24
(levers: bridge-up share in first sieges must hold) — grouped into one batch per wave.

---

### Estimated effort

| Item | Sessions |
|---|---|
| Wave 0 (W0-A, W0-B) | 2 |
| O01 | 1.5 |
| O02 | 3.5 |
| O03 | 5 |
| O04 | 4 |
| O05 | 5 |
| O06 | 3 |
| O07 | 4 |
| O08 | 6 |
| O09 | 4 |
| O10 | 4 |
| O11 | 4 |
| O12 | 3.5 |
| **Total** | **49.5** |

With the waves above and 6-8 agents, the calendar is about 4 waves (≈ 10-12 agent-session slots
end to end, plus four coordinator batches).

### Cross-milestone notes

- **One memory store (`saga.rs`).** O05's reel weights, O06's life story, O07's journal and drift,
  O09's episodes and O12's inspirations all read "what happened to whom". M3's C01 (inner life),
  C02 (grief and vengeance), C05 (family sagas) and M8's P01 should write into it rather than
  adding per-dwarf memory vectors (the `Dwarf` struct stays lean; `thoughts` stays the short-term
  list).
- **One place store (`places.rs` + `names.rs`).** Named halls (O03), battlefields (O09), player
  names (O11), roads, works (O12), and later M4's B17 world names and M8's outposts. Name with a
  hash, never the game rng, so names never shift batches.
- **The overseer as a person (`reign.rs`).** Faith is the "opinion of the leader" store M3's C06
  (council), C04 (fort society) and M8's P02 (crown and succession) need; succession logic
  should be shared with nobles' succession, not duplicated.
- **Hooks instead of edits.** `m2hooks.rs`/`ov_m2.rs` (one line per system), `links_<item>` in the
  brain, `lines_<item>` in the chronicle, `fit_<kind>` in `ov_fit`, `key_m2` in app: the same
  pattern serves M3-M8 and keeps `Game::step`, `brain`, `write_chronicle` and `key_normal` from
  being merge battlegrounds. Consider generalising `m2hooks` to a `systems.rs` registry
  (`[(&str, fn(&mut Game), cadence)]`) used by `step`, with the per-system timing feeding F3.
- **LogEntry with place and kind.** Announcement control, sounds, the director and the export
  all need it; M5's war reports and M7's dread events will too.
- **The facing frame.** O02's `rel`/`local` should be used by every future overseer construction
  (M5's siege engines, M6's water works) — never `base.offset` again.
- **Refactors worth doing first:** W0-A (facing), W0-B (fields and hooks), the gatehouse and
  glyph moves, `ov_fit` split, chronicle/brain helpers. Also from M1: S10's tunables file should
  exist before M2 so faith thresholds, wealth tiers, gate-stage triggers and dig hardness live
  there and the batch can sweep them.
