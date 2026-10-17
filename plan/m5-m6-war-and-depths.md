# Implementation plan: Milestone 5 (war in earnest) and Milestone 6 (deep fortress)

Grounded in the DwarfClone tree as of 2026-10-17 (`src/game/combat.rs` 2044 lines, `military.rs`,
`expeditions.rs`, `disasters.rs`, `water.rs`, `fire.rs`, `weather.rs`, `farming.rs`, `animals.rs`,
`traps.rs`, `src/path.rs`, `src/world.rs`, `src/terrain.rs`, `src/game.rs`). These milestones assume
Milestones 1-4 have landed. Where an item leans on one of them (S03 captives, S07 metal bolts,
C02 vengeance vow, B02 diplomacy, B18 missions, B20 vermin, O04 defences in the plan), that is said
in the item, along with a fallback if it hasn't landed.

**What the code does today (the parts these milestones change)**

- **Combat** is `Game::strike(att: &Fighter, def: &Fighter, body)` (combat.rs). The hit chance is
  `0.55 + 0.05*(skill diff)`, a shield blocks 25%, damage is `dmg*roll + skill*0.5 - armour`.
  `Fighter` has no attributes, morale or fatigue. `dwarf_fighter` takes its skill from
  `d.fight_level()`, which is `fight_xp/400`: one combat skill for everything.
- **Nerve** exists only in `creature_tick`. A siege creature leaves once it has lost 60% of its
  blood, and an animal once it is `badly_hurt`. `rout()` fires when the general dies or when half
  the army is dead. Dwarves never break. Civilians `Flee` (`combat_awareness`, `do_flee`) and only
  hit back if they were struck in the last 40 ticks.
- **Siege AI** (`creature_think`, the `c.siege` branch) works in three steps. First, a Dijkstra
  search (`path::search`, `SEARCH_LIMIT`) for any dwarf. If none is reachable, it batters a
  `DOOR_LOCKED` door (`c.bash`, 250 ticks, `break_door`). Otherwise it loots goods lying outside
  or wanders. Raised bridges (`BRIDGE_RAISED`) and floodgates are impassable (`World::passable`)
  and can't be battered, so a raised bridge makes the fort untouchable.
- **Sieges** are rolled once a season in `step` (`tick % SEASON == 0` → `maybe_siege`), so they
  always arrive on day 1 of a season, from `edge_entry()` (a random edge tile), seen a day ahead
  (`incoming`).
- **Squads** (`military.rs`) have `order: Train|Station|Kill`, a `UniformWeapon` and an
  `armor: bool`. There are no schedules, ammo pools, alerts or burrow orders. Bolts go into a
  per-dwarf `ammo` quiver (`do_equip`).
- **Expeditions** take the dwarves off the map (`Expedition.members: Vec<Dwarf>`). The outcome is
  one roll, `p = strength/(strength + 3*danger)`. There are no captives, retaliation or tribute.
- **Disasters** are only `check_ceiling`/`cave_in`, a 5-span rule on `Tile.dug` that always deals
  a 7-point wound. Falls do no damage (`update_dwarf` just drops `pos.z`). Water and magma are
  cellular, 0-7 deep (`fluids`, `magma`, `aquifers`), and there are no pumps or power.
  Lighting doesn't exist: `creature_active` hides cave dwellers only on unrevealed tiles, and
  `ui.rs:422` draws every creature on a revealed tile.
- **Caverns** are 2-3 z-bands (`World.caverns`), populated once (`populate_caverns`: 2 spiders and
  3 troglodytes a layer). They have pools and 7% `TileKind::Tree` "mushroom trees" that can
  already be chopped like any tree. `cavern_floors()` scans every tile of every band (a
  performance trap once D01 calls it more often).
- **Farming:** a farm tile's `farm` counter rises by 1 every 10 ticks to `RIPE=3200`. A harvest
  yields `1 + (skill>=3) + coin`. There is no soil, water or fertility. Pastures and breeding
  exist (`tame_tick`, `HERD_CAP`), but animals don't graze or give products.
- **Hauling** is `search_haul` (game.rs), one item per trip with containers absorbed. It is
  already a large hot function.

---

## Shared foundations (build these before the parallel waves)

These are the interface contracts. Each is small and lives in its own new file, so it can land
in one short session (wave 0) and every dependent item can then work against a fixed API.

### F-EV: the event queue (`src/game/events.rs`, new)
Several items want to react when something dies, a siege ends, a trap fires or a weapon kills:
A01 cheer, A02 morale, A03 tallies and named battles, D01 irritation, D07 scars, the overseer.
Today each would add lines to `creature_died`, `kill`, `rout` and `spring_trap`, which are the
worst merge hot spots in the tree. Add one queue instead:

```rust
#[derive(Clone, Debug)]
pub enum Ev {
    Slain { victim: u32 /*creature id*/, species: Species, by: Option<u32> /*dwarf*/, weapon: Option<u32>, pos: Pos, siege: bool, general: bool },
    DwarfDied { id: u32, by: Option<u32> /*creature*/, cause: String, pos: Pos },
    Wounded { dwarf: Option<u32>, creature: Option<u32>, part: Part, grade: Wound },
    SiegeBegan { idx: usize }, SiegeEnded { idx: usize, outcome: &'static str },
    Routed { side: Side /*Ours|Theirs*/, squad: Option<u32> },
    TrapSprung { trap: u32, victim: u32 },
    Noise { pos: Pos, loud: u8 },          // digging, felling, smashing (D01)
    Breach { pos: Pos, how: Breach },      // B06
}
pub enum Side { Ours, Theirs }
// Game field:  #[serde(skip)] pub(crate) events: Vec<Ev>,
impl Game {
    pub(crate) fn emit(&mut self, e: Ev);   // push
    pub(super) fn drain_events(&mut self);  // called once at the end of step(); dispatches to
                                            // each subscriber: fn on_event(&mut self, e: &Ev)
}
```
Subscribers are one line each in `drain_events` (`self.morale_on(&e); self.legend_on(&e); ...`),
so a new item adds one line and its own function in its own file. The queue isn't saved
(`serde(skip)`), because it is drained every tick.

### F-STEP: a step registry (refactor in `game.rs`)
`Game::step` is about 150 lines of `if tick % N == k { call }`. Split it into
`fn step_fluids`, `fn step_daily` (the `tick % DAY == 0` block), `fn step_seasonal` (the
`tick % SEASON == 0` block) and `fn step_periodic`, keeping one call per system. Every new system
in M5/M6 adds **one line** in the right block. This is pure motion with no behaviour change,
proved by the existing `year_with_wildlife_and_caverns_runs` and a 1-year seed comparison of
`annals` (same deaths and sieges).

### F-NERVE: morale (`src/game/morale.rs`, new; used by A01, A02, A04, A06, B03, D06)
```rust
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Nerve { pub fear: i16 /*0..=1000*/, pub state: NerveState }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NerveState { #[default] Steady, Shaken, Broken /*runs*/, Frenzied /*vengeful*/, Yielded }
impl Game {
    pub(crate) fn courage_of_dwarf(&self, d: &Dwarf) -> i32;    // personality.anxiety, attributes (W20), epithet, sergeant (A04)
    pub(crate) fn courage_of_creature(&self, c: &Creature) -> i32; // SpeciesInfo + general alive
    pub(crate) fn frighten(&mut self, who: Who, by: i16, why: &str);
}
pub enum Who { Dwarf(u32), Creature(u32) }
```
Add `#[serde(default)] pub nerve: Nerve` on both `Dwarf` and `Creature`. That is one field each,
the whole footprint in dwarf.rs and creature.rs.

### F-ATTR: attributes (`src/dwarf.rs` + `src/game/attributes.rs`, new; W20)
```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Attrs { pub str: u16, pub agi: u16, pub tough: u16, pub endur: u16, pub will: u16, pub focus: u16 } // 0..=2000, 1000 = average
impl Default for Attrs { fn default() -> Self { /* all 1000 */ } }
// Dwarf: #[serde(default)] pub attrs: Attrs, #[serde(default)] pub weapon_xp: [u32; 6] /*Axe,Sword,Spear,Hammer,Crossbow,Unarmed*/, #[serde(default)] pub rust: [u16; 6]
// Fighter gains: pub str: f32, pub agi: f32, pub tough: f32 (1.0 = average)
```
Old saves load with average attributes, and `weapon_xp` is seeded from `fight_xp` on load
(`Game::fix_after_load`).

### F-CAPT: captives (`src/game/captives.rs`, new; used by A01, A02, B03, A06)
If the S03 "captives" step or B18 has already built a store, adopt this shape. Otherwise create it:
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Captive { pub id: u32, pub who: Held, pub civ: Option<u32>, pub since: u64, pub holding: Holding }
pub enum Held { Foe { species: Species, label: Option<String>, figure: Option<u32> }, Ours(Box<Dwarf>) }
pub enum Holding { Cage(u32 /*item id*/), Chained(Pos), AtSite(u32 /*overworld site*/) }
// Game: #[serde(default)] pub captives: Vec<Captive>
```
Cage traps already turn a creature into a `Cage` item labelled with its name (`spring_trap`).
F-CAPT keeps the creature data (species, label, figure) instead of throwing it away, so freeing,
ransoming, interrogating and executing become possible.

### F-HP: structure damage (`src/game/structures.rs`, new; B06, A05, D04, F05)
```rust
// Game: #[serde(default)] pub damage: HashMap<Pos, u16>   (sparse: only battered tiles)
impl Game {
    pub(crate) fn toughness(&self, p: Pos) -> u16;  // by TileKind + mat + built_from; door/bridge by Building.mat
    pub(crate) fn batter(&mut self, p: Pos, force: u16, by: &str) -> bool; // true = it broke; dig_version += 1
}
```
`break_door` becomes a call to `batter`. A reinforced wall is `built_from == Some(ItemKind::Bar)`
(a construction from two blocks and a bar), so `Tile` needs no new field.

### F-LIGHT: the light map (`src/game/light.rs`, new; B13, D01, D06, D10)
```rust
// Game: #[serde(skip)] light: Option<(u64 /*light_version*/, Vec<u8>)>   (0 dark .. 7 bright)
impl Game {
    pub fn light_at(&self, p: Pos) -> u8;
    pub(crate) fn seen(&self, by_dwarves: bool, p: Pos) -> bool; // lit, or within 2 tiles of a dwarf
    pub(crate) fn light_dirty(&mut self);                         // bumps light_version
}
```
The map is recomputed lazily, only for z-levels that have dwarves on them (a per-level cache
keyed on `dig_version + light_version`).

### F-SIEGEPATH: obstacle-aware planning (`src/siege_path.rs`, new; B06, A05, D01 raids)
This is the main path.rs change. `path::search`/`astar` stay untouched: they are the hot path,
and their `Came` arrays and `thread_local SPARE` can be reused. Add:
```rust
pub enum Breach { Walk, Dig(Pos), Climb(Pos), Span(Pos) /*lay floor*/, Batter(Pos), Ram(Pos) }
pub struct SiegeCaps { pub dig: bool, pub climb: bool, pub build: bool, pub batter: u16 /*force*/, pub avoid: Vec<(Pos, u32)> /*learned danger*/ }
pub fn plan(world: &World, start: Pos, goal: impl FnMut(Pos)->bool, caps: &SiegeCaps, hp: &dyn Fn(Pos)->u16, limit: usize)
    -> Option<(Pos, Vec<(Pos, Breach)>)>;
```
`plan` is a Dijkstra like `path::search`, but its neighbour function adds costed virtual edges.
Natural wall: Dig, cost 60 + hardness/4. Rough wall face (`smooth == 0`): Climb, cost 40.
Open air or channel gap one tile wide: Span, cost 120. Door, raised bridge or floodgate:
Batter, cost `hp(p)/force*10`. `avoid` tiles get a surcharge. It runs **once per army** every
300 ticks or when `dig_version` changes, from the general's position, never once per goblin.
Members follow the leader's plan (`Game.siege_plan: Option<SiegePlan>`, `serde(skip)`, rebuilt
on load).

---

## Milestone 5: war in earnest

**Order and why**

1. **F-EV, F-STEP, F-NERVE, F-ATTR, F-CAPT, F-HP** (wave 0, about 3 sessions in parallel). Every
   M5 item touches `creature_died`, `strike` or `Dwarf`; with these in place they don't collide.
2. **W20 Attributes** comes first among the items. It changes the fight maths (`strike`,
   `Fighter`) that A02 morale and A04 drills balance against. Rebalancing after A02 would mean
   doing it twice.
3. **A02 Blood and nerve** and **A01 Sieges with surprises** run in parallel. A01 lives in
   `maybe_siege`/`lay_siege_of`/`traps.rs`; A02 lives in `morale.rs`, `combat_awareness` and
   `creature_tick`'s breaking block.
4. **A03 Legends of the blade** (event subscriber) and **A04 Squad life** (military.rs) run in
   parallel once A02 has landed: A04 needs nerve for discipline, A03 needs `Ev::Slain`.
5. **B06 Siege AI** is the critical path (L, high risk). It starts in wave 1 on F-SIEGEPATH and
   F-HP, because it is mostly new files. **A05 Engines of war** needs B06's `batter` and breach
   actions.
6. **A06 When all is lost** needs A02 (levy and martial law live there), A01's floodgate step
   and B02 diplomacy (M4) for surrender.
7. **B03 Armies abroad** comes last. It needs F-CAPT, A02 (yield → prisoners), W20 (tactics), and
   M4's B18 (captives left behind, retaliation). If B18 hasn't landed, B03 absorbs OUT-01/03.

Critical path: F-HP → F-SIEGEPATH → B06 (6 sessions) → A05 (4) → batch. Everything else fits
around it.

**Merged duplicates** (build each once): SIE-20 "drowning the invaders" = OVR-17 "floodgates as
weapons" (A01 builds the mechanism, A06 the desperate use). MIL-07 levy = OVR-07 martial law
(A02 builds it, A06 adds the "last stand" trigger). MIL-37 = HLT-32 (W20). MIL-13 revenge
builds on C02's vengeance vow (M3) if it exists. MIL-19 rescue extends B18's OUT-02.

**Release check**
- `cargo test --release` green, with every step's test listed below.
- Batch `--autoplay --runs 25 --years 3 --seed 5000` against the M4 baseline:
  - Siege arrival days spread over all 84 days of a season (a histogram in report.rs).
  - ≥30% of sieges against a fort with a raised bridge record a breach (`SiegeNote.breach`).
  - ≥1 rout of a dwarf squad and ≥1 goblin taken prisoner per 10 sieges.
  - Survival after year 3 is no worse than −10 points against the M4 baseline. The goal is
    drama, not a massacre; B06 is the risk here.
- To watch: `SEED=5005 STYLE=warlord SIEGE=3 cargo test siege_replay -- --ignored --nocapture`
  shows goblins digging or bridging around a raised bridge, and a ballista crew firing.

### W20 Attributes and skill decay  (effort: 2 sessions; risk: med)
- **Design:**
  - F-ATTR as above. Attributes are rolled in migrant and child creation: `Dwarf::new` and
    `migrants()` (game.rs) roll a normal around 1000, and `family.rs` gives children their
    parents' mean ± noise.
  - `dwarf_fighter` fills `Fighter.str/agi/tough` from them; `creature_fighter` fills them from
    `SpeciesInfo.scale`.
  - `strike` changes: hit chance gains `+0.08*(att.agi - def.agi)`, damage is multiplied by
    `att.str.powf(0.7)`, and the wound is reduced by `def.tough` (`Body::wound` takes
    `dmg/tough`). Bleeding and recovery use toughness and endurance.
  - Weapon skill: `weapon_xp[kind]` replaces `fight_xp` for the held weapon's class.
    `fight_level()` becomes the max over classes, so `squad_strength`, the heroes code and the
    UI keep working.
  - Growth: use raises an attribute (str from melee and hauling, agi from dodging and training,
    endurance from long walks), capped per day.
  - Rust: `attrs_daily()` (one line in `step_daily`) lets unused weapon xp decay after a season
    without use, to at most −1 rank, as DF does, and attributes drift back towards their base.
- **Steps:**
  1. `Attrs` field, rolled for new dwarves and inherited by children, shown on the dwarf sheet
     (ui.rs, one block). Test: `attributes_are_rolled_and_inherited` (family.rs test helper).
  2. Fighter uses attributes. Test: in `arena`, two dwarves with equal xp but str 1400 vs 600
     against a goblin, 200 duels each: the strong one wins ≥15 points more often (the "Done when").
  3. Per-weapon xp: `weapon_xp` and the migration of old saves. Test:
     `axe_practice_does_not_teach_the_crossbow`, plus a save round-trip test (an old JSON
     without `attrs` loads).
  4. Growth and rust. Test: `skills_rust_after_a_season_idle_but_not_below_one_rank`.
- **Overseer & screen:** `ov_army` drafts by `fight_score = level*2 + (str+agi+tough)/600` instead
  of raw `fight_xp`, and the brain notes "Drafted the strongest." The dwarf sheet shows the six
  attributes as words (very strong … weak), and the arena prints them.
- **Measure:** arena win rates (step 2). In the batch, goblins killed per siege go up ≥10%, and
  bleeding stays under a quarter of deaths (the S07 target must hold).
- **Risks / open questions:**
  - Changing strike maths shifts all of S07's balance, so re-run the arena table in WISHLIST.
  - Should animals get attributes? No. Species scale is enough.
- **Parallel work:**
  - **Writes:** `src/game/attributes.rs` (new: rolling, growth, rust, `attrs_daily`);
    `dwarf.rs`: `Dwarf` +3 fields, `fight_level`; `combat.rs`: `Fighter`, `dwarf_fighter`,
    `creature_fighter`, `strike`; `military.rs: try_shoot` (the Fighter literal).
  - **Reads / depends on:** F-ATTR only.
  - **Hot spots:** dwarf.rs (3 fields), combat.rs `strike` (this item owns it in wave 1;
    serialize with anything else touching `strike`), `step_daily` (+1 line), ui.rs dwarf sheet
    (+1 block).
  - **Agent size:** one agent, two sessions.

### A02 Blood and nerve  (effort: 5 sessions; risk: high)
- **Design:**
  - A **fear model** in `morale.rs`. Every 10 ticks, for fighters in combat:
    `fear += wounds*k1 + allies_down_nearby*k2 + outnumbered*k3 - courage`, and it decays when no
    threat is near. The thresholds move `NerveState` from Steady to Shaken (−15% hit chance in
    `strike`, so a frightened shooter's hands shake), then to Broken (the dwarf `Flee`s; the
    creature `leaving = true`).
  - **Berserk:** if temper ≥ 30 and fear > 800, a dwarf turns `Frenzied` (double attack rate, no
    fleeing) instead of breaking.
  - **Leaderless armies (MIL-09):** in `on_event(Ev::Slain{general:true})`, each army member rolls
    loyalty (a new `Creature.loyal: u8`). The loyal turn `Frenzied` for 200 ticks, the rest go
    straight to `Broken`. This replaces the immediate `rout()` call in `creature_died`: `rout`
    becomes "≥60% of the army Broken".
  - **Dwarf squads:** when a squad leader (the first member, or the A04 sergeant) falls, the same
    roll happens using personality.
  - **Rallying charge (MIL-10):** a Steady dwarf with fear < 200 and an epithet, or will > 1300,
    next to Broken squadmates resets their fear by −400 and adds a deed through `earn_epithet`
    ("Rallier", RANK_FEAT).
  - **Emergency levy (MIL-07):**
    `pub fn levy(&mut self, reason: &str) -> usize` drafts able adults into a temporary
    `Squad { levy: true }` (new field, serde default) armed by `equip_soldier` with *anything*
    (`UniformWeapon::Any` accepts picks, axes and also tools); it stands down after the siege.
    **Martial law:** `Game.martial_law: Option<u64>` stops breaks and festivals; everyone gets a
    −stress thought per day and a resentment memory when it lifts.
  - **Cornered civilians (MIL-08):** in `combat_awareness`, a civilian who is fleeing with no
    escape (`do_flee` found no path, and they are adjacent to a threat) grabs the best item within
    2 tiles (`find_item` with limit 30) and attacks. If they hold a tool (pick or axe) they fight
    at once.
  - **Bystanders (MIL-17):** when a dwarf within 5 tiles is struck, each civilian weighs
    `regard(victim)` and their temper against their fear: join, flee or freeze. Don't join if the
    attacker is a friend (for brawls from C03).
  - **Yield and POWs (MIL-16):** a Broken creature that `person()` and can't reach the edge within
    a path of 60, or is adjacent to ≥2 dwarves, sets `Yielded`. It stops attacking and drops its
    weapon. The nearest dwarf decides using temper and grudges (C02, if present). Mercy turns it
    into an F-CAPT `Captive { holding: Chained(pos) }` taken to a jail or barracks (justice.rs
    already has jailing for dwarves, so reuse its spot finder). Otherwise it is executed (a
    memory, and a −grudge from that civ). 10% of yields are fakes that strike again.
  - **Mutiny (MIL-15):** in `morale_daily`, a squad whose mean stress is above a threshold, or
    which is starving or lost its last fight, may grumble (thoughts), refuse a station order
    (`squad_task` returns false), or desert (the dwarf leaves the map with gear, as in
    `expedition_returns` in reverse).
- **Steps:**
  1. Fear and `NerveState` for creatures. `creature_tick`'s breaking block uses `nerve.state ==
     Broken`. Test: `a_goblin_alone_against_three_breaks_before_it_bleeds_out`.
  2. Fear for dwarves: soldiers who break flee, and the shaken miss more. Test:
     `a_lone_recruit_breaks_against_five_goblins_but_a_veteran_holds`.
  3. Leader falls cuts both ways (both sides). Test:
     `killing_the_general_sends_some_into_frenzy_and_the_rest_running` (with a seeded rng;
     replaces the old instant rout assertion in
     `enemies_from_history_march_on_the_fort_and_break_when_their_general_falls`).
  4. Rallying charge. Test: `a_brave_dwarf_turns_a_broken_squad_back`.
  5. Levy and martial law: the `levy()` API and the `Squad.levy` flag. Test:
     `a_levy_arms_cooks_with_whatever_is_at_hand_and_stands_down_after`.
  6. Cornered civilians fight back. Test: `a_cornered_cook_grabs_a_knife` (wait, there are no
     knives yet: a pick or axe within 2 tiles, else a chair as a 2.0-damage club via a new
     `ItemKind::weapon_damage` entry for furniture).
  7. Bystanders choose sides. Test: `a_friend_of_the_victim_joins_and_a_stranger_flees`.
  8. Yield and prisoners. Test: `a_broken_goblin_surrounded_by_dwarves_yields_and_is_chained`
     (asserts `captives.len()==1`).
  9. Mutiny and desertion. Test: `a_starving_losing_squad_refuses_orders_and_one_deserts`.
- **Overseer & screen:**
  - `ov_alert` calls `levy()` when `hostile siege count > 2 × soldiers` and the gate is breached.
    A Cautious overseer levies early, a Bold one never, and the brain notes "Armed the cooks."
  - New trouble `Trouble::Mutinous(squad)`: the overseer answers with ale and a feast, or
    discharges the ringleader.
  - The dwarf list shows a nerve glyph (`!` shaken, `~` broken, `*` frenzied).
  - Spotlight cards for a squad that breaks, a rally and a yield. The chronicle counts
    "prisoners taken".
- **Measure:** per siege (new `SiegeNote` fields `routs_ours`, `routs_theirs`, `yielded`): ≥1
  goblin yield per 10 sieges, and dwarf-squad routs in 10-30% of sieges. Deaths "in the shelter"
  fall below 25% (cornered civilians).
- **Risks / open questions:**
  - Soldiers breaking could undo M1's survival gains. Tune courage so trained squads in armour
    rarely break (they get courage from `armor.len()` and squad size).
  - Should execution of POWs be allowed under a non-ruthless overseer? Suggest it's only the
    victor's personality, and the overseer can later "release" them.
- **Parallel work:**
  - **Writes:** `src/game/morale.rs` (new; most of the item), `combat.rs`: `creature_tick`
    (breaking block), `combat_awareness`, `do_flee` (the "no escape" return), `rout`,
    `creature_died` (remove the direct rout; emit `Ev`); `military.rs`: `Squad.levy`,
    `UniformWeapon::Any`, `squad_task` (refusal); `captives.rs` (POW holding);
    `overseer.rs: ov_alert` (one levy call); `troubles.rs` (+1 variant).
  - **Reads / depends on:** F-NERVE, F-EV (`Ev::Slain{general}`), F-CAPT (`Captive`,
    `Holding::Chained`), W20 (`Attrs.will` in courage; default 1000 if not landed).
  - **Hot spots:** combat.rs (it owns `combat_awareness` and the breaking block in this wave),
    overseer.rs `ov_alert` (+5 lines), annals.rs `SiegeNote` (+3 serde-default fields),
    report.rs (+1 line per field).
  - **Agent size:** too big for one session. Split into **A02a** (steps 1-4, morale core:
    one agent, 2 sessions) and **A02b** (steps 5-9, levy, civilians, POWs, mutiny: one agent,
    3 sessions, after A02a).

### A01 Sieges with surprises  (effort: 3 sessions; risk: low)
- **Design:**
  - **Arrival (SIE-02):** `maybe_siege` moves out of the seasonal block. At the season's start it
    rolls *whether* a siege comes and picks a day `SEASON_START + rng(0..SEASON - DAY)` stored in
    `Game.siege_due: Option<u64>` (serde default). `step_periodic` checks it every 25 ticks, the
    way `army_arrives` does now.
  - **Edge spread:** `edge_entry()` takes the foe site's direction on the world map
    (`overworld.sites[foe.site]` relative to `self.site`), so armies come from their home side
    ±45°.
  - **Ambush (SIE-42):** `lay_siege_of` gains `ambush: bool`. Ambushers spawn with
    `sneaking = true` (the field exists for skulkers and hides them in `ui.rs:422`) and no scout
    warning. They reveal when a dwarf sees them within 4 tiles (`skulker_think` already has that
    test), or after 300 ticks. Chance is about 25%, higher for goblins. `SiegeNote.ambush: bool`.
  - **Freeing the caged (SIE-04):** siege members whose path passes within 8 tiles of a `Cage`
    item with `Captive.civ == their civ` divert, work 150 ticks, and the captive respawns hostile
    (from F-CAPT's species and label).
  - **Victory cheer (SIE-03):** on `Ev::SiegeEnded{outcome:"routed"}`, every dwarf gets a −20
    stress thought ("cheered the breaking of the siege"). The overseer's `ov_daily` may queue a
    feast: festivals.rs `start_feast(reason)` if it exists, else a half-day break with drinks for
    the squad that killed most.
  - **Jammed traps (SIE-19):** in `spring_trap`, count corpses and carcasses on the trap tile
    (`items` at pos). With ≥2, the trap misfires and a "jammed" building flag
    (`Building.jammed: bool`, serde default) is set. The `Dump`/haul code already moves corpses,
    so `find_haul` priorities make "clear a jammed trap" a high-priority haul even under alert
    (soldiers' cover permitting).
  - **Drowning invaders (SIE-20):** `pub fn floodgate_trap(&mut self, lever: u32) -> FloodPlan`.
    It finds floodgates linked to a lever, checks water behind them, and returns the tiles that
    would flood. The overseer only does it in A06; A01 provides the machinery and a player
    "flood when 3+ hostiles are in the zone" lever setting (`Building.auto_flood: Option<u8>`).
- **Steps:**
  1. Random day and directional edge. Test: `sieges_arrive_on_varied_days_and_from_the_foes_side`
     (force 20 rolls, assert ≥5 distinct days and that most entries lie on the foe's half).
  2. Ambushes. Test: `an_ambush_comes_unannounced_and_is_revealed_when_seen`.
  3. Victory cheer and feast. Test: `breaking_a_siege_lifts_every_dwarf`.
  4. Jammed traps and clearing them. Test: `a_trap_choked_with_bodies_stops_working_until_cleared`.
  5. Freeing the caged. Test: `goblins_break_their_caged_comrade_out` (needs F-CAPT).
  6. The floodgate machinery and auto-flood lever. Test: `an_auto_flood_lever_drowns_a_corridor_full_of_goblins`.
- **Overseer & screen:**
  - The alert now has to cope with no warning: `ov_alert` already reacts to `danger` from
    visible creatures.
  - Feasts are a new overseer decision (Revel ambition boosted by `Sense::Besieged` decaying
    after victory).
  - Spotlight cards "Ambush!", "The siege is broken" (existing) plus a feast card.
  - The report adds an arrival-day histogram and ambush share.
- **Measure:** the arrival histogram is flat across the season. Ambush sieges kill ≤1.5× open
  sieges (a guard against the ambush being too lethal). The share of victory feasts is visible
  in the batch.
- **Risks / open questions:** ambushes on young forts could wipe them out, so no ambush for the
  first siege (S03 raiding party). Migrants and sieges no longer share a day (good).
- **Parallel work:**
  - **Writes:** `combat.rs`: `maybe_siege`, `edge_entry`, `lay_siege_of`, `creature_think`
    (siege branch: the cage diversion); `traps.rs`: `spring_trap`, `floodgate_trap` (new fn),
    `traps_tick`; `buildings.rs`: `Building` +2 fields; `game.rs`: `step` (move the
    `maybe_siege` call, +1 periodic line), `search_haul` (jammed priority: a small predicate);
    new `src/game/victory.rs` (cheer and feast subscriber).
  - **Reads / depends on:** F-EV (`SiegeEnded`), F-CAPT (step 5 only), F-STEP.
  - **Hot spots:** game.rs `step` (2 lines) and `search_haul` (1 predicate). The `creature_think`
    siege branch is shared with B06, so A01 lands first or B06 rebases.
  - **Agent size:** one agent, 3 sessions. Step 5 can wait for F-CAPT.

### A03 Legends of the blade  (effort: 3 sessions; risk: low)
- **Design:**
  - **Fight log and named battles (MIL-12):** `annals.rs` gains
    `battles: Vec<Battle { name, siege: Option<usize>, start, end, place: Pos, deeds: Vec<String>, dead: Vec<String> }>`.
    A battle opens on the first blow (`note_blow` with a siege, or ≥3 combatants within 10 tiles)
    and closes 200 ticks after the last blow. Its name is generated from the place (the room name
    from O03, or "the Gate", "the Third Hall") and the foe: "the Slaughter at the Gate".
    A ring buffer `combat_log: VecDeque<String>` (serde skip, 500 lines) feeds a new log pane
    (ui.rs), with war cries from personality.
  - **Named weapons (MIL-11):** `Item.kills: u16` and
    `Item.history: Option<Box<Vec<String>>>` (serde default). `Ev::Slain{weapon:Some}` increments
    it. At 5 kills, or one general or megabeast kill, the weapon gets a name
    (`dwarf::artifact_name`) and `quality = max(quality, 5)`, and goes into the legends. The
    wielder becomes attached (`Dwarf.attached: Option<u32>`), with a −thought when it's taken
    away.
  - **Kill records (MIL-67):** `Dwarf.kill_list: Vec<(u64, String /*what*/, Option<String> /*named victim*/)>`
    (capped at 20 plus a count), shown on the sheet.
  - **Nemesis (MIL-13):** on `Ev::DwarfDied{by:Some(c)}`, the closest kin and friends
    (`relations`) get `Dwarf.nemesis: Option<Nemesis { creature: Option<u32>, species: Species, vow: bool }>`.
    In combat, `do_attack`'s target choice prefers the nemesis, and fear from pain is halved
    (the hatred dulls their pain). If C02's vow exists, reuse it and only add the species hatred.
  - **Champion (MIL-44):** a noble position `Position::Champion`, appointed by the monarch or
    baron (positions.rs). It gives +courage to all soldiers, and a dead champion is a fort-wide
    −thought.
  - **Favourite weapons (MIL-41):** `Dwarf.fav_weapon: Option<ItemKind>` for a third of dwarves.
    Holding it gives +1 skill in `dwarf_fighter`; being re-equipped away from it brings a
    grumble, and a refusal if `attached`.
- **Steps:** 1 the battle log and named battles (test: `a_siege_fight_is_logged_and_named`) ·
  2 weapon tallies and names (test: the "Done when": `a_weapon_that_kills_a_general_earns_a_name_and_history`) ·
  3 kill lists on the sheet (test: `kills_are_listed_with_named_victims`) · 4 nemesis (test:
  `a_dwarf_whose_brother_fell_goes_after_that_goblin`) · 5 champion (test:
  `the_monarch_appoints_one_champion`) · 6 favourite weapons (test:
  `a_dwarf_fights_better_with_their_favourite_and_grumbles_without_it`).
- **Overseer & screen:**
  - `equip_soldier` respects `attached` and `fav_weapon` (an overseer ranking tweak).
  - The overseer appoints the champion when a noble exists (`ov_posts`).
  - The chronicle's year page lists the named battles. The legends list named weapons. The
    unit sheet has a "Deeds" tab. The O05 highlight reel can jump to a battle's place.
- **Measure:** in a 3-year batch, ≥1 named weapon per surviving Warlord fort, and battles named
  in ≥80% of sieges.
- **Risks / open questions:** item history costs save size, so cap it at 8 lines. Battle naming
  needs room names (O03). Fall back to compass words ("the North Stair").
- **Parallel work:**
  - **Writes:** new `src/game/blades.rs` (all subscribers, naming, nemesis);
    `annals.rs` (`Battle`, `battles`); `items.rs`: `Item` +2 fields; `dwarf.rs`: `Dwarf`
    +4 fields (`kill_list`, `nemesis`, `fav_weapon`, `attached`); `combat.rs: dwarf_fighter`
    (+1 line fav bonus), `do_attack` (target preference: 3 lines); `positions.rs`
    (+`Champion`); ui.rs (log pane, sheet tab).
  - **Reads / depends on:** F-EV `Ev::Slain{by, weapon}`, `Ev::DwarfDied{by}`; C02 vow (optional).
  - **Hot spots:** dwarf.rs fields (append-only, at the end of the struct), ui.rs (a new pane fn
    in its own module `ui_log.rs` if possible).
  - **Agent size:** one agent, 3 sessions.

### A04 Squad life  (effort: 5 sessions; risk: med)
- **Design:**
  - **Routines (MIL-26):** `Squad.routine: [Duty; 12]` (one per month: `Duty::{Train, Ready /*station*/, OffDuty, Patrol(u32), Defend(u8 burrow)}`)
    plus fort `Alert` levels: `Game.alerts: Vec<Alert { name, routine_override: Duty }>` and
    `Game.active_alert: usize`. `squad_task` reads `effective_duty(squad)` instead of `order`.
    `SquadOrder` stays as the "now" override (Kill still wins).
  - **Eating in turns:** a member who leaves to eat or sleep is covered by the next member (only
    half the squad may be on needs while Ready).
  - **Patrols (MIL-27):** `Game.routes: Vec<Route { id, name, points: Vec<Pos> }>`. A soldier
    walks them with `Task::Station` hops.
  - **Defend a burrow (MIL-28):** a soldier with `Defend(b)` attacks any hostile whose tile has
    burrow bit `b`. `combat_awareness` gets a burrow check (`t.burrows & bit`).
  - **Gear off duty (MIL-29):** when `OffDuty`, armour is dropped into the barracks (an armour
    stand or chest, i.e. a stockpile in the barracks zone). On alert they re-equip via
    `equip_soldier`, so a festival alarm catches them half-dressed.
  - **Ammo pools (MIL-30):** `Squad.ammo: Vec<AmmoRule { mat: Option<Mat>, count: u16, use_: AmmoUse::{War, Train, Both} }>`.
    `wants_bolts` and `is_bolts` take the rule into account: training draws Train bolts (wood,
    bone), war draws metal. The quiver becomes `Dwarf.ammo` plus `ammo_mat: Mat`.
  - **Saved uniforms (MIL-58):** `Game.uniforms: Vec<Uniform { name, weapon: UniformWeapon, mat: Option<Mat>, head, body, shield: bool }>`.
    `Squad.uniform: Option<usize>` replaces `weapon/armor`, which are kept and read when it is
    `None` for old saves. `equip_soldier` prefers low `wear`.
  - **Teaching (MIL-38):** in the barracks, a soldier ≥3 ranks above an adjacent recruit
    "demonstrates". The recruit gains xp × (1 + teacher level/4) with no blows. A lazy student
    (diligence < −25) learns at half rate.
  - **Sparring accidents (MIL-39):** the sparring pair rolls `strike` with damage × 0.2. A
    reckless pair (temper > 25 both) uses × 0.6. A rare gash, with a medic job and a grudge.
  - **Discipline (MIL-40):** `Squad.sergeant: Option<u32>`. The sergeant's personality picks a
    style: harsh (−stress on recruits, but +courage), inspiring (+courage, a good thought) or soft.
    It feeds `courage_of_dwarf`.
- **Steps:** 1 routines, alerts and cover rotation (test: the "Done when" first half,
  `a_squad_follows_its_monthly_routine`) · 2 patrol routes (test: `patrollers_walk_their_route`) ·
  3 defend a burrow (test: `defenders_drop_everything_when_a_wolf_enters_their_burrow`) ·
  4 gear off duty (test: `off_duty_soldiers_rack_their_armour_and_reequip_on_alarm`) · 5 ammo
  pools (test: the "Done when" second half, `training_burns_wooden_bolts_and_war_draws_iron`) ·
  6 uniforms (test: `a_saved_uniform_applies_to_two_squads_and_old_saves_keep_theirs`) ·
  7 teaching (test: `a_veteran_teaches_a_recruit_without_blows`) · 8 sparring accidents (test:
  `reckless_sparring_partners_hurt_each_other_more`) · 9 sergeant (test:
  `a_harsh_sergeant_keeps_a_shaky_squad_in_line`).
- **Overseer & screen:**
  - `ov_army` builds a default routine by style: Warlord has more Ready months, Steady trains in
    winter. It sets an alert "Siege" that `ov_alert` toggles, replacing its direct
    `order_station` loop. It creates a War/Train ammo split once metal bolts exist (S07).
  - The military screen gets a routine grid (a month × squad table) and an ammo column.
  - The overseer's notes say "Called the Siege alert."
- **Measure:** bolts fired per siege up ≥50% (ammo pools keep quivers full). Deaths during
  festivals are tracked; the share in sieges that start mid-festival should rise only modestly.
- **Risks / open questions:**
  - Cover rotation could leave the gate empty. Test with the replay.
  - The UI for routines is large, so add a minimal grid first.
- **Parallel work:**
  - **Writes:** `military.rs` (most of it: `Squad` fields, `squad_task`, `go_train`,
    `drill_xp`, `wants_bolts`, `is_bolts`); new `src/game/routines.rs` (Duty, Alert, Route,
    Uniform); `combat.rs: equip_soldier, do_equip` (bolt material); `overseer.rs: ov_army,
    ov_alert` (replace the station loop with the alert switch); app.rs/ui.rs military screen.
  - **Reads / depends on:** F-NERVE (sergeant courage, A02a); S07 metal bolts (else only wood:
    the pools still work).
  - **Hot spots:** overseer.rs `ov_alert` (conflicts with A02's levy line, so A02b lands first),
    `combat.rs: equip_soldier` (also read by A03 fav weapon, a 1-line overlap).
  - **Agent size:** too big for one session. Split into **A04a** (steps 1-4: routines, patrols,
    burrows, gear) and **A04b** (steps 5-9: ammo, uniforms, teaching, sparring, sergeant). They
    run sequentially, or in parallel if A04b doesn't touch `squad_task`.

### B06 Siege AI  (effort: 6 sessions; risk: high)
- **Design:**
  - F-SIEGEPATH and F-HP. The army gets a `SiegePlan { goal: Pos, steps: Vec<(Pos, Breach)>, made: u64, ver: u64 }`
    computed for the general (or the first member). In `creature_think`'s siege branch, the
    "no dwarf reachable" fallback (where it today searches for locked doors) becomes:
    1. Use the plan's first non-Walk breach.
    2. Walk to the stand tile next to it.
    3. Perform the breach for N ticks (a `c.bash`-like `Creature.work: Option<(Pos, Breach, u32)>`,
       replacing `bash` and `bash_work`, with a serde alias for old saves).
    4. Mutate the world:
       - Dig: `TileKind::Floor`, boulder dropped.
       - Span: `TileKind::Floor` with `built_from = Some(Log)`.
       - Batter: `self.batter(p, force)`.
       - Climb: no world change. Climbing is a movement edge only for caps.climb movers, via a
         new `Mover::Climber`, so `World::neighbors` allows going up a rough wall face beside an
         open tile, at cost 30.
  - Then `dig_version += 1` (the reach and path caches invalidate correctly already).
  - **Who can do what:**
    - `SpeciesInfo` gains `breach: u8` bits (DIG, CLIMB, BUILD, BATTER).
    - Trolls and ogres (new Species in the goblin army composition, 1 per 5 goblins after
      siege 2) batter and dig.
    - Goblins climb and build.
    - Humans build, dwarf raiders dig, elves climb.
  - **Wall-breakers (SIE-10):** batter force by scale. A raised bridge gets
    `toughness = mat × 400`, so it can be battered, slowly.
  - **Reinforced walls (SIE-40):** a new construction recipe in construct.rs, "Reinforced wall"
    (2 blocks + 1 bar), with toughness ×4. A smoothed wall can't be climbed (`smooth >= 1`).
  - **Sappers (SIE-11):** the Dig cost is high, so the planner digs only when there's no cheaper
    path ("only after every other route has failed").
  - **Learning (SIE-14):** `Game.siege_memory: HashMap<u32 /*civ*/, Vec<(Pos, u16)>>` (serde
    default) records death and trap tiles. The next plan adds `avoid` costs, so they come in
    another way, dig around a trap corridor, or climb.
  - **Cut-off leaders (SIE-39):** when the plan's cost is above budget (`limit`), the general
    chooses: loot (existing), burn (`ignite` a wooden building or tree near the fort), camp (A05)
    or withdraw.
  - **Battering rams (SIE-15):** a ram is a Creature (`Species::Ram`, `tame: false`, `siege`),
    moved only when ≥3 crew members stand adjacent (crew are ordinary goblins with
    `role: Crew(ram_id)`). Its batter force is ×5 against doors and bridges.
  - **Feints (SIE-34):** a goblin army with ≥8 members whose first assault took losses pulls
    back 15 tiles (`leaving = true` for 150 ticks) and sets an ambush group `sneaking` near the
    gate. If soldiers chase out, the ambushers reveal.
  - A Warlord overseer can feint too: the A04 alert "Lure" stations a squad outside, then
    withdraws behind the traps.
- **Steps:**
  1. F-HP: `damage`, `toughness`, `batter`. Doors via `batter`. Test: `a_troll_batters_a_door_faster_than_a_goblin`.
  2. Batter raised bridges and walls; reinforced-wall construction. Test:
     `a_raised_wooden_bridge_falls_to_trolls_but_a_reinforced_wall_holds_longer`.
  3. `siege_path::plan` and the army plan cache. Test (pure path test in `siege_path.rs`): on a
     synthetic world, the planner prefers walk < batter < dig, and returns Dig only when walls
     are the only way.
  4. Diggers and builders execute breaches. Test (the "Done when"):
     `an_army_facing_a_raised_bridge_digs_or_builds_its_way_in` (on `flat_patch` with a moat
     and a raised bridge: within 2 days a breach and a hostile inside).
  5. Climbers (`Mover::Climber` in `World::neighbors`) and smoothing defeats them. Test:
     `goblins_climb_a_rough_wall_but_not_a_smoothed_one`.
  6. Siege memory and avoiding traps. Test: `the_second_army_avoids_the_trap_corridor_the_first_died_in`.
  7. Cut-off leaders adapt: burn, camp or withdraw. Test: `a_shut_out_army_sets_the_woodpile_alight`.
  8. Rams with crews. Test: `a_ram_needs_its_crew_and_breaks_the_gate`.
  9. Feints and fall-backs (theirs, then the Warlord's). Test:
     `soldiers_who_chase_a_feint_meet_the_ambush`.
- **Overseer & screen:**
  - **Defence awareness:** `ov_defences` reinforces the gate walls once bars are in stock, and
    smooths the outer walls (it already has smoothing jobs via O03).
  - A new trouble `Trouble::Breach(pos)` makes the overseer station squads at the breach, wall
    it up after the siege, and add the breach tile to its own "weak points" list for O04.
  - The brain gets a "Breach" sense that boosts Fortify.
  - On screen: work-in-progress breaches show as `%` and digging as `×`. Announcements read
    "The goblins are tunnelling under the east wall!", with a spotlight on the first breach.
- **Measure:**
  - `SiegeNote.breach: Option<String>` (dig, climb, bridge, ram, door).
  - Target: ≥30% of sieges against raised bridges breach.
  - **Performance:** `one_year_timing` within +10% (plans are per army per 300 ticks;
    `SIEGE_PLANS` counter like `SEARCHES`).
- **Risks / open questions:**
  - **Biggest balance risk in M5:** forts that only survived because of the bridge will fall
    again. Mitigate: breaching takes days (time for the overseer to respond), only later sieges
    bring trolls, and a shut-out army still leaves after `stay = DAY*5`.
  - **Performance:** the planner's virtual edges widen the search through rock. Cap it at 60k
    nodes and use the A* heuristic toward the fort's `base`.
  - **Decision for the user:** should sappers dig through *constructed* walls? Proposed: only
    battering, no digging.
- **Parallel work:**
  - **Writes:** new `src/siege_path.rs`, new `src/game/structures.rs`, new
    `src/game/siege_ai.rs` (plan cache, breach execution, memory, feints, rams);
    `world.rs`: `Mover::Climber` + a branch in `neighbors` (≈10 lines); `creature.rs`:
    `SpeciesInfo.breach`, new species `Troll`/`Ram`, `Creature.work`/`role`; `combat.rs`:
    `creature_think` siege branch (replace the locked-door fallback with
    `self.siege_fallback(c)`, one call), `break_door` (call `batter`), `lay_siege_of` (trolls and
    rams in the army); `construct.rs` (reinforced wall).
  - **Reads / depends on:** F-HP, F-SIEGEPATH, F-EV (`Ev::Breach`); A01 (it lands first in
    `creature_think`).
  - **Hot spots:** world.rs `neighbors` (the hottest function in the game: the Climber branch
    must be behind `if m == Mover::Climber` so other movers pay nothing), combat.rs
    `creature_think` (a one-call footprint).
  - **Agent size:** too big for one session. Split into **B06a** (steps 1-4: HP, planner,
    dig/build/batter, 3 sessions), then **B06b** (steps 5-6: climbers, memory) and **B06c**
    (steps 7-9: adapt, rams, feints) in parallel after B06a.

### A05 Engines of war  (effort: 4 sessions; risk: med)
- **Design:**
  - A new `BuildingKind::SiegeWorkshop` makes parts. New buildings `Ballista` and `Catapult`
    (3×3, have a `facing: u8`, `Building.ammo: u16`, built from parts).
  - **Ballista (SIE-16):** an operator standing on the "back" tile fires along `facing` (a
    straight line, range 20, via `clear_shot` generalised to any length). Each bolt passes
    through up to 3 creatures (`strike` with dmg 25, edged) and stops at walls. Fortifications
    don't stop it.
  - **Catapult (SIE-17):** targets a tile in a 90° arc at range 8-25, with vertical reach ±1 z.
    A stone (from `Boulder` stock) lands with a scatter radius of 2 and shatters: `strike` dmg 15
    on everything in a 1-tile radius, plus `batter` on structures (so it damages enemy rams and
    camps).
  - **Crews (SIE-54):** a default work detail (`Labor::Siege`, a new labor, with
    `labors: [bool; NUM_LABORS]` padded via the existing `pad_labors`). Operators stay at their
    post while the "Siege" alert is on. An engines list on the military screen shows each
    engine's state.
  - **Attackers' engines (SIE-55):** armies after siege 3 bring a ram (B06) and sometimes a
    catapult `Creature` (`Species::Catapult`, crewed like rams) that bombards from range 15 at
    the gate, and `batter`s walls.
  - **Siege camps (SIE-22):** a long siege (`stay` extended to `DAY*20` for big armies)
    establishes `Game.siege_camp: Option<Camp { pos, tent: Pos, since }>`. The army rests and
    heals there, turns out when soldiers sally (alert radius 20), and sends 2-goblin search
    parties after any dwarf outside. The fort is starved: caravans don't come, and migrants are
    turned back. If food runs out, dwarves may eat corpses (`Task::Eat` on a `Corpse`, with a
    horror thought), as an option for the overseer under starvation.
- **Steps:** 1 siege workshop, parts and the ballista building (test:
  `a_ballista_bolt_passes_through_a_line_of_goblins`) · 2 crews and posts (test:
  `crew_stays_at_the_ballista_during_alert_and_leaves_after`) · 3 catapult (test:
  `a_catapult_stone_shatters_and_hurts_a_cluster`) · 4 attackers' ram and catapult (test, the
  "Done when": `a_ballista_on_the_wall_kills_a_ram_crew_before_it_reaches_the_gate`) ·
  5 siege camps and the long siege (test: `a_big_army_camps_and_sends_searchers_after_a_woodcutter`).
- **Overseer & screen:**
  - Warlord and Fortify ambition build a ballista facing the approach once mechanisms and logs
    allow. It adds `"ballista"` to `Overseer.done` and assigns crew via labor.
  - Under a camp, a new `Sense::Encircled` boosts Muster and Provision, and A06's
    desperate choices.
  - On screen: the engines list, bolt-line flashes (reuse the dragon breath line render), a
    "Siege camp" spotlight, and camp tents drawn on the map.
- **Measure:** the share of siege kills by engine (a new `BY_ENGINE` index in
  `SiegeNote.blows`, a 6th slot via serde default on the array: use a `Vec` or a new field).
  Forts survive a long siege ≥50% of the time.
- **Risks / open questions:**
  - Long sieges can stall a batch fort for a season (fine: that's drama), but they need a cap.
  - Eating the dead is grim. Offer a flag `--no-cannibalism` for the user? Decision for the
    user.
- **Parallel work:**
  - **Writes:** new `src/game/engines.rs` (buildings, firing, crews, camps); `buildings.rs`:
    `BuildingKind` +4 (SiegeWorkshop, Ballista, Catapult… appended at the end of the enum and the
    `ALL` array), `Recipe` +3; `creature.rs` +1 species; `labor.rs` +1 labor;
    `annals.rs` +1 field.
  - **Reads / depends on:** B06a (`batter`, Ram species and crew role), F-HP, A04a (alerts, for
    crew posts; otherwise use `self.alert`).
  - **Hot spots:** buildings.rs enum and `ALL` (also touched by D08 and W26: coordinate by
    appending only, and land A05's enum lines in its own commit early), ui.rs build menu.
  - **Agent size:** one agent, 4 sessions. Step 5 is separable.

### A06 When all is lost  (effort: 3 sessions; risk: med)
- **Design:**
  - **Floodgates as weapons (OVR-17):** the overseer uses A01's `floodgate_trap`. On a style with
    O04 defences it plans a "drowning hall" (a floodgate from a cistern or river channel into the
    entrance corridor). When ≥3 hostiles are in it and no dwarves are, a Ruthless or desperate
    overseer pulls. With a magma source (`near_magma`) a Warlord can do the same with magma.
    Under the alert, it locks civilians out first.
  - **Last stand (OVR-07):** `desperation()` =
    `hostiles_inside / defenders + civilians_dead_this_siege/pop`. Above a threshold it calls
    A02's `levy()` plus `martial_law`, and a spotlight "The last stand". Afterwards everyone
    resents it (C06 council, if present, takes a favour hit).
  - **Surrender and occupation (OVR-20):** when desperation is extreme and the overseer is
    Cautious, or Greedy (it saves the hoard), it can surrender to a *civilized* foe (not goblins
    with a bloodlust trait; B16's evil armies, if landed). The army stops attacking (all
    `nerve = Yielded`-like "parley" state) and leaves 5 occupiers.
    `Game.occupation: Option<Occupation { civ, overlord: String, since, tribute_due: u64, garrison: Vec<u32> }>`.
    Every season, tribute (wealth × 10%) is hauled to the depot and taken. The garrison
    patrols. The fort resents it (a daily −thought). An uprising becomes possible when soldiers
    > garrison × 2, or a hero arises: the overseer gets the ambition **Overthrow** (a new
    `Ambition`, gated by `Sense::Occupied`). Freedom comes with a feast and a legend; a failed
    uprising brings executions.
- **Steps:** 1 drowning hall in the overseer plan and its use (test, the "Done when" part 1:
  `a_desperate_overseer_floods_the_entrance_hall`) · 2 last stand (test:
  `an_overwhelmed_overseer_arms_everyone`) · 3 surrender and occupation (test, part 2:
  `a_cautious_overseer_surrenders_to_humans_and_pays_tribute`) · 4 the uprising (test:
  `a_strong_occupied_fort_rises_and_throws_out_the_garrison`).
- **Overseer & screen:**
  - This item *is* the overseer's. `Trait` reactions: Stubborn never surrenders, Sentimental
    won't flood with dwarves nearby, Greedy surrenders sooner.
  - Brain senses: `Desperate`, `Occupied`.
  - On screen: a status line "Occupied by the Kingdom of X (tribute due in 12 days)", plus
    spotlight cards and chronicle lines ("the year of the yoke").
  - The report gets a "surrendered" outcome counted separately from "fell".
- **Measure:** in the batch, ≥1 flood-kill and ≥1 surrender over 25 forts × 3 years, and
  surrendered forts survive the year ≥70% of the time (otherwise surrender is a trap).
- **Risks / open questions:**
  - The game-over logic (`alive_count()==0`) is unaffected, but autoplay `Trial` outcomes need a
    new state.
  - **Decision for the user:** can the *player* surrender too (a menu item)? Proposed: yes.
- **Parallel work:**
  - **Writes:** new `src/game/last_resort.rs` (desperation, floods, surrender, occupation,
    uprising); `overseer.rs`: `ov_alert` (+1 call `self.ov_last_resort(o)`), `ov_defences`
    (+1 call for the drowning hall plan); `ambitions.rs`: +`Overthrow`, +2 senses;
    `report.rs`/`overseer.rs Trial` (outcome).
  - **Reads / depends on:** A01 `floodgate_trap(lever) -> FloodPlan`; A02b `levy(reason)`,
    `Game.martial_law`; B02 (M4) civ stance for who accepts surrender (fallback: never goblins);
    F-NERVE.
  - **Hot spots:** ambitions.rs `Ambition::ALL` and `Sense::ALL` arrays (append only; also
    touched by A05 `Encircled` and D02 `Greed`: make these appends their own tiny commits).
  - **Agent size:** one agent, 3 sessions.

### B03 Armies abroad  (effort: 5 sessions; risk: med)
- **Design:**
  - The mission model changes from one roll to a short simulated **campaign**:
    `Expedition` gains `mission: Mission::{Raid{loot: LootPref}, Raze, Rescue(u32 captive), Recover(u32 artifact), Strike(u32 figure), Shield(Pos /*world cell*/), Conquer, Levy}`
    and `log: Vec<String>`. `expedition_returns` becomes `resolve_mission`. It runs 1-3 off-map
    "clashes", each rolled from `strength × tactics_mult` against the site's defenders. Wounds,
    deaths and captures are per clash (so the report reads as a story).
  - **Tactics (MIL-45):** `Labor::Tactics`-like xp in `weapon_xp`'s neighbour,
    `Dwarf.tactics_xp: u32`. The best tactician's level plus `attrs.focus` gives
    `tactics_mult = 1 + 0.08 × level`. It is trained by missions and named in the report.
  - **Raid vs raze (MIL-18):** a raid brings loot by `LootPref` (metal, goods, food) and
    captives (F-CAPT foes). A raze ruins the site in the overworld (`site.ruined = true`; the
    civ loses pop) and brings little loot but big fear in that civ (B02 stance).
  - **Rescue (MIL-19):** targets an `Ours` captive held `AtSite(s)` (from B18/OUT-01, or from the
    A02 "carried off" path). Success brings them home with a trauma memory (C01).
  - **Recover treasure (MIL-20):** for artifacts stolen by `Skulker` thieves. Today `creature_tick`
    deletes stolen loot at the edge; instead record `Game.stolen: Vec<(item snapshot, civ, site)>`.
  - **Strike at leaders (MIL-21):** assassinate or capture a figure. On success,
    `overworld.kill_figure` and the next siege from that civ is weaker or delayed (and named in
    the warning: "seeking revenge for their lord").
  - **Field armies (MIL-23):** a squad posted to a world cell between the fort and an enemy. An
    `incoming` army passing it fights a clash first: it may be turned back, delayed, or arrive
    with fewer and with a 2-day warning instead of 1.
  - **Conquest, tribute, barony (MIL-61):** a conquered site gets an administrator (a dwarf
    removed to the overworld), and sends yearly tribute (a caravan-like delivery,
    `update_caravan` reuse). Two conquered sites raise the fort to a barony (positions.rs Baron
    arrives; a C06 hook).
  - **The crown's levy (MIL-22):** a monarch mandate (psyche.rs `issue_mandate` style) demands N
    soldiers for a season. Obeying sends them off (`send_expedition` with `Mission::Levy`, and
    they return as veterans or not); refusing costs favour (C06) and the crown's goodwill (B02).
  - **Retaliation:** a raid or raze raises that civ's siege chance and size (`maybe_siege`
    weights by `grudge_against_us`). If B18 OUT-03 exists, reuse it.
- **Steps:** 1 campaign resolution with a mission log, and tactics xp (test:
  `a_mission_log_reads_as_clashes_and_names_the_tactician`) · 2 raid vs raze with chosen loot
  and captives (test, the "Done when": `a_raid_brings_back_loot_and_a_captive_and_retaliation_follows`) ·
  3 rescue missions (test: `a_rescue_brings_the_captive_home`) · 4 recover stolen treasure
  (test: `a_stolen_artifact_is_recovered_or_rumoured`) · 5 strike at leaders (test:
  `killing_their_warlord_weakens_the_next_siege`) · 6 field armies (test:
  `a_posted_squad_delays_an_army_and_sends_word`) · 7 conquest, tribute and barony (test:
  `conquering_two_sites_makes_a_barony_and_tribute_arrives`) · 8 crown's levy (test:
  `refusing_the_levy_costs_favour`).
- **Overseer & screen:**
  - The Venture ambition picks a mission by trait: Greedy raids, Bold strikes at leaders,
    Sentimental rescues, Warlord conquers. `Sense::Captives` (ours held abroad) boosts Venture.
  - The levy mandate becomes a trouble the overseer obeys or refuses by trait.
  - On screen: the mission picker on the military screen (`e`), a mission report card (the log),
    the world map showing posted field armies and conquered sites, and chronicle lines.
- **Measure:** expedition deaths stay < 5% of all deaths (the S06 target); ≥1 captive
  returned home per 10 forts; retaliation sieges are tagged in `SiegeNote.cause`.
- **Risks / open questions:**
  - Overlap with B18 (M4). Whoever lands second adapts. This plan assumes B18 has built `Captive`
    and `AtSite`.
  - Only one expedition can be away (`expedition: Option<_>`). Field armies need a
    `Vec<Expedition>`: migrate with `#[serde(default)] pub expeditions: Vec<Expedition>`, and
    read the old `expedition` on load.
- **Parallel work:**
  - **Writes:** `expeditions.rs` (most of it); new `src/game/missions.rs` (campaign clashes,
    conquest, tribute); `combat.rs: maybe_siege` (grudge weight, field-army intercept: 2 small
    hooks), `creature_tick` (stolen-loot record: 3 lines); `psyche.rs` (levy mandate);
    `dwarf.rs` +1 field; `overworld.rs` (`site.conquered_by`, serde default).
  - **Reads / depends on:** F-CAPT; A02b (yield → POW); W20 (`attrs.focus`); B18/B02 (M4)
    stances, captives `AtSite`; `Game.expeditions: Vec<_>` migration (do it in step 1).
  - **Hot spots:** combat.rs `maybe_siege` (A01 rewrote it, so land after A01), overseer.rs
    (Venture choice in ambitions.rs `begin_ambition`).
  - **Agent size:** too big for one session. **B03a** (steps 1-4: campaign, raid/raze, rescue,
    recover) and **B03b** (steps 5-8: strike, field armies, conquest, levy), sequential.

---

## Milestone 6: deep fortress

**Order and why**

1. **Foundations:** F-LIGHT (for B13, D01, D06), and `src/game/fluids.rs` — a split of
   `fluids()` out of game.rs into water.rs or a new file, so W26, F05 and B12 don't fight over
   game.rs. Add a `Game.noise` accumulator (D01) as a subscriber of `Ev::Noise`.
2. **Wave 1** (disjoint files):
   - B13 (light.rs)
   - F05 (disasters.rs: quakes, melt, surge)
   - D05 (new disease.rs)
   - D07 (body.rs, new scars.rs)
   - D08 (buildings.rs recipes)
   - D10 (needs.rs)
   - D11 (weather.rs)
   - D09 (animals.rs)
3. **Wave 2:**
   - W26 (pumps and power: needs the fluids split, and F05 for eruptions shared with ENG-26)
   - D01 (needs B13 for dark caverns and F-EV noise)
   - D04 (disasters.rs supports: after F05, same file)
   - D06 (needs B13 for blind-in-dark, and D04 for cave-in survival)
   - B11 (hauling; independent but large, so give it its own agent early)
4. **Wave 3:**
   - B12 (needs W26 channels for irrigation and F05 floods)
   - D03 (needs B12 underground wood and D01 cavern life)
   - D02 (needs D01 and W26 magma creatures; the underworld comes last)

Critical path: fluids split → W26 → B12 → D03. D02 is the climax, last.

**Merged duplicates:** ENG-26 eruptions = F05 "magma sea surges". DIG-20 magma dwellers = W26
"magma creatures". AGR-08 flood farming = F05 spring melt (B12 only adds the fertility bonus).
DIG-08 dark caverns = HLT-36 blind in the dark = B13 (one light model; D01 and D06 add only
their reactions). DIG-10 rich cavern soil = AGR-03 soil depth (B12 builds it, D03 references
it). DIG-18 fungus forests = AGR-22 underground wood (D03 builds it, B12 references it).
CRF-14 toys (D08 makes them) + MND-46 play (D10 uses them).

**Release check**
- `cargo test --release` green. `one_year_timing` within +15% of M5 (light and fluids are the
  risk).
- Batch:
  - A new death-cause spread: falls, disease, cold and cave-ins each appear but none is >10%.
  - ≥50% of 3-year forts open a cavern and log or farm it (D03).
  - ≥1 pump stack or water wheel per Builder fort.
  - Survival no worse than −10 points against M5.
- To watch: a Delver fort over 3 years: warnings, the deep metal, the underworld breach (D02).

### F05 Disasters  (effort: 3 sessions; risk: med)
- **Design:** three disasters in `disasters.rs`, plus `step_seasonal` hooks:
  - **Earthquakes:** rare, once per 2-5 years, more often on a VOLCANO site.
    `quake(epicentre)` cracks rough natural walls within r=12 (30% of `Wall` tiles adjacent to
    open space get `F-HP` damage). Unsupported dug spans are re-checked via `check_ceiling` with
    a lower span (so a quake can drop a borderline hall). With 20% chance it opens a fissure: a
    1-tile column from the current deepest dug level into the nearest cavern band, which counts as
    a breach (`self.breaches.push`, so the overseer hangs a door: existing code in
    `ov_defences`).
  - **Spring melt:** at `seasonal_ice(0)`, on sites with snow (`Tile.snow` count > N) or a
    river, add water: river tiles +2 for 3 days. The lowest open dug tiles connected to the
    surface within 10 z get `water += 2` once if they are below the river level
    (`world.water_level`). It is limited to tiles reachable through open stairs and ramps from a
    flooded surface tile (a bounded flood fill, ≤5000 tiles).
  - **Magma surge:** once per 1-3 years on magma maps. The sea level at z=1-2 rises 1-2 levels for
    a season via `magma = 7` injection on sea-edge tiles, then recedes (removal of injected
    cells). `magma_cells = None`.
- **Steps:** 1 earthquakes and fissures (test: `an_earthquake_cracks_walls_and_can_open_a_cavern`) ·
  2 spring melt flood (test, the "Done when": `a_spring_flood_fills_a_low_hall_and_dwarves_flee_it`;
  fleeing needs `combat_awareness`-like flight from water ≥4: add `Task::Flee` from deep water in
  `update_dwarf`'s drowning branch) · 3 magma surge (test:
  `the_magma_sea_surges_a_level_and_falls_back`).
- **Overseer & screen:**
  - A new trouble `Trouble::Flooded(room)`: the overseer forbids the room, then pumps it (W26)
    or channels it out.
  - `Sense::Danger` fires on a surge near forges.
  - The Cautious trait builds a floodgate on the lowest stair (it reuses floodgate placement).
  - On screen: spotlights "Earthquake!", "The spring melt floods the lower halls", "The magma sea
    rises", and chronicle lines (`year_log.quake` etc.).
- **Measure:** deaths by drowning ≤3% of all deaths. ≥1 flood event per 25-fort batch in
  river forts.
- **Risks / open questions:**
  - A flood fill per spring on big maps costs too much. Bound it, and run it only at the season
    tick.
  - Quakes damaging the overseer's plan: use `breaches` and `Trouble` so it repairs.
- **Parallel work:**
  - **Writes:** `disasters.rs` (+3 fns); `weather.rs: seasonal_ice` (+1 call);
    `water.rs: magma` (surge injection helper); `game.rs: update_dwarf` drowning branch (flee:
    3 lines); `chronicle.rs YearLog` +3 serde-default bools.
  - **Reads / depends on:** F-HP (cracks); F-STEP.
  - **Hot spots:** game.rs `update_dwarf` (3 lines), disasters.rs (D04 later edits the same
    file, so F05 lands first).
  - **Agent size:** one agent, 3 sessions.

### B13 Lighting  (effort: 4 sessions; risk: high (performance))
- **Design:**
  - F-LIGHT. Sources:
    - outside by day → 7, at night → 2. A day/night clock: `tick % DAY`. There is none today,
      so add `Game::is_night()`.
    - `BuildingKind::Torch` (wall-mounted, from wood or coal) → 5, radius 5.
    - `Lamp` (metal + tallow/oil) → 6, radius 7.
    - `fire > 0` → 6. Magma → 4. Workshops with fuel (furnace and forge) → 4.
    - Gem windows: a construction `Window(mat)` that passes daylight into a room, coloured light
      for the UI.
  - Light falls off linearly and is blocked by `solid`. It is computed per z-level that has
    dwarves on it, cached on `light_version` (bumped by building or removing a source, a fire
    tick, dawn and dusk) and `dig_version`.
  - **Seeing:** `creature_active` keeps its meaning. A new `Game::visible(c)` = lit ≥2 at
    `c.pos`, or a dwarf within `2 + light` tiles. `ui.rs:422` filters by it, and `threats_near`
    for civilians only sees visible threats (so a creature in the dark gets close).
  - **Ambush from the dark:** an unseen attacker's first blow gets +0.2 hit chance and ignores
    the shield.
  - **Dislike of the dark:** work in light <2 gets −20% speed (`Self::labor` takes a multiplier)
    and a mild thought once a day.
  - **Fire as a tool and threat (ENG-05):** a designation `Desig::Burn` on trees and shrubs.
    Bucket brigades: `Task::Douse { fire: Pos }` fetches water by bucket from a well or river and
    sets `fire = 0` on arrival. Burning buildings make dwarves inside flee (the HLT-13 link in D06).
  - **Hearths and chimneys (ENG-50):** a `Hearth` building warms a room (for D06's cold). It
    needs a `Chimney` (an open tile above, or a dug shaft to the surface). Without one, it makes
    miasma-like smoke (reuse `Tile.miasma` with a smoke flag) → coughing thoughts.
  - **Fluid heat (ENG-52):** water within 1 tile of magma becomes "warm" (D06 cold relief).
    Water falling onto magma (`quench`) gives a steam puff: a `fire`-like burn for 1 tick,
    scalding anyone in it.
- **Steps:**
  1. Light map, day and night, sources from sky, fire and magma, a light overlay key. Test: `a_cavern_is_dark_and_a_torch_lights_it`.
  2. Torch and lamp buildings plus recipes. Test: `a_lamp_needs_oil_or_tallow` (once D08 adds oil; tallow exists).
  3. Visibility: unseen creatures hidden, threat sensing by light. Test, the "Done when":
     `a_creature_in_an_unlit_cavern_is_unseen_until_a_dwarf_is_close`.
  4. Ambush from the dark and the dislike of working in it. Test:
     `an_unseen_spider_strikes_first_and_miners_work_slower_in_the_dark`.
  5. Fire as tool: burn designation and bucket brigades. Test: `a_bucket_brigade_puts_out_a_burning_workshop`.
  6. Hearths and chimneys, plus fluid heat and steam. Test: `a_hearth_without_a_chimney_smokes_out_the_room`.
- **Overseer & screen:**
  - The overseer lights its halls: `ov_fit` places a torch per room, and lamps in the halls once
    tallow is in stock. It lights the cavern breach door.
  - A trouble `Lack::Light` (new) when dark-work complaints exceed N.
  - The UI dims unlit tiles (a new `dim(color, light)` in ui.rs render, behind an `L` toggle for
    the overlay).
  - The bucket brigade is visible as a stream of `Douse` tasks, and an announcement reads
    "Dwarves form a bucket line."
- **Measure:**
  - Performance: `one_year_timing` +≤8%. A light recompute counter in the F3 overlay.
  - Cavern deaths from ambush rise (that's intended). Cave horror kills per fort stay ≤ baseline
    +20%.
- **Risks / open questions:**
  - **Performance:** recomputing a 200×200 level on every fire tick. Mitigate by recomputing
    incrementally only around changed sources (a dirty rectangle).
  - **Decision for the user:** night on the surface? It doubles the visual change. Proposed: yes,
    but only dims, it doesn't hide.
- **Parallel work:**
  - **Writes:** new `src/game/light.rs`; new `src/game/hearth.rs` (hearths, smoke, steam);
    `buildings.rs` +4 kinds (Torch, Lamp, Hearth, Window); `fire.rs`: `Desig::Burn` handling, a
    `Douse` task (`fire.rs` new fns); `dwarf.rs: Task::Douse`; `combat.rs: threats_near`
    (visibility filter: 1 line), `strike` callers (ambush bonus via a `Fighter.unseen: bool`);
    `game.rs: labor()` multiplier (1 line); ui.rs (dim and overlay).
  - **Reads / depends on:** F-LIGHT; F-STEP; tallow/oil (D08 for oil).
  - **Hot spots:** ui.rs map render (the dim call: one line in the tile loop), buildings.rs enum
    (append), dwarf.rs `Task` enum (append).
  - **Agent size:** too big for one session. **B13a** (steps 1-4, light and sight, 2 sessions)
    and **B13b** (steps 5-6, fire tool, hearths and heat, 2 sessions), parallel after step 1's
    API.

### W26 Water and magma works  (effort: 5 sessions; risk: med)
- **Design:**
  - **Screw pumps (ENG-21):** a 1×2 `Pump` building with an intake tile (`pos`) and an output
    `pos + facing` one level up. When powered or manned (`Task::Pump`, a dwarf works it), it
    moves 1 unit/3 ticks of water or magma from below the intake to the output tile, through
    `fluids()`. Stacks chain naturally.
  - **Power (ENG-28):** `src/game/power.rs` with `Network { nodes: Vec<u32>, supply: i32, demand: i32 }`.
    Sources: `WaterWheel` (needs flowing water: a tile whose water changed in the last 40 ticks,
    known from the `water_cells` active list), `Windmill` (outside, weather-scaled). Links:
    `Axle` and `Gear` buildings. Consumers: pumps (10), millstone (existing, +speed), and future
    rollers. Networks are rebuilt only when a power building changes (`power_version`).
  - **Aquifer tools (ENG-15):**
    - Embark warning: the embark screen already shows `params.aquifer`, so add "heavy aquifer"
      in red and a confirm.
    - Damp and warm warnings when digging next to `aquifer` or magma tiles: the dig code in
      `game.rs` calls `self.warn("damp", …)`, using the existing `near_magma`.
    - Pumping a shaft dry: a heavy aquifer seeps 0.5/25 ticks per leak (`aquifers()`), so a pump
      stack outpaces it.
  - **Wells and cisterns (ENG-18):** a well over a cistern fed by a channel. `can_place_well`
    already requires a channel over water. Add the overseer plan: a cistern room below the dining
    hall fed by a floodgated channel from a river or aquifer.
  - **Fresh, salt and stagnant (ENG-37):** a Tile `wq: u8` (bit 1 salt, bit 2 stagnant,
    `skip_serializing_if` zero). Coast and ocean water is salt at generation (terrain.rs COAST).
    Still water away from flow for 1 season goes stagnant. Drinking salt gives a −thought and
    ½ thirst relief; stagnant risks sickness (a D05 hook). Mixing with fresh flow clears it.
  - **More sources (ENG-38):** rain barrels (a `Barrel` outside under rain fills up as a `Drink`
    of water: a new `label = "water"`, like buckets), filling barrels at streams (a job), and
    pools that refill with rain (`weather_daily` adds +1 to outside pools when it is wet).
  - **Magma works (ENG-27):** channels already move magma. Add a magma "pour" (a floodgate over
    open air) for A06's use, and magma forges powered by a pumped stack (the existing
    `magma_below`).
  - **Magma creatures (DIG-20):** `Species::FireImp`/`MagmaCrab` live in the sea (they walk on
    magma via `Mover::Magma`) and come up when a pump stack or channel connects the sea to dug
    tiles. They ignite things (`ignite`).
  - **Volcanoes refill (ENG-40):** on VOLCANO sites, the crater column's magma tops up +1 per day
    to its generated level.
  - **Eruptions (ENG-26):** shared with F05's surge. A VOLCANO site variant spills from the
    crater at the surface.
- **Steps:** 1 pumps, manned (test, the "Done when": `a_pump_stack_drains_a_flooded_shaft`) ·
  2 power networks: wheel, windmill, axles and gears (test:
  `a_water_wheel_powers_a_pump_through_axles`) · 3 aquifer warnings and the embark confirm (test:
  `digging_beside_an_aquifer_warns`) · 4 cistern plan (test: `the_overseer_builds_a_cistern_and_well_below_the_dining_hall`) ·
  5 water quality (test: `salt_water_quenches_less_and_stagnant_water_sickens`) · 6 more sources
  (test: `a_rain_barrel_fills_in_the_rain`) · 7 magma works, creatures and refill (test:
  `pumping_magma_up_lets_a_fire_imp_out`).
- **Overseer & screen:**
  - The Builder and Architect styles: a Delve or Prosper ambition step "Bring magma up" builds a
    pump stack to the forges. Floods (`Trouble::Flooded`) get a pump answer.
  - The O02 plan includes a cistern.
  - On screen: a pump glyph with an animated arrow, power numbers in the building's `k` look, and
    a "power" overlay.
- **Measure:** water-related deaths (drowning, magma) ≤3%. Thirst deaths don't rise
  (stagnant water). Timing +≤5%.
- **Risks / open questions:**
  - Pumps inside `fluids()` must keep its active-list optimisation: a pump re-adds its tiles to
    `water_cells`.
  - **Decision for the user:** do minecarts (B11) also use power (rollers)? Proposed: B11 owns
    rollers and consumes `power.rs`.
- **Parallel work:**
  - **Writes:** new `src/game/power.rs`, new `src/game/pumps.rs`; `water.rs` (magma creature
    hooks, refill, quality); `world.rs: Tile` +1 field (`wq`), `Mover::Magma` (neighbors
    branch); `buildings.rs` +5 kinds; terrain.rs (salt on COAST); `fluids()` moved out (wave 0
    refactor) + 1 pump call.
  - **Reads / depends on:** the fluids split (wave 0); F05 surge helper (shared magma injection).
  - **Hot spots:** world.rs `Tile` (B12 also adds a field there: coordinate by appending, both
    `skip_serializing_if`), `neighbors` (magma mover branch, guarded), buildings.rs enum.
  - **Agent size:** too big for one session. **W26a** (steps 1-3, pumps, power, aquifers) and
    **W26b** (steps 4-7, cisterns, quality, sources, magma), sequential.

### D01 The living caverns  (effort: 3 sessions; risk: med)
- **Design:**
  - **Irritation (DIG-01):** `Game.cavern_noise: [u32; 3]` per band (serde default). It is fed by
    `Ev::Noise`, emitted from digging within or near a band (in the dig job), felling cavern
    trees, and dumping into chasms or rivers. It decays 2% a day.
  - **Raids (DIG-03):** `maybe_cavern_raid()` in `step_seasonal`, with probability ∝ noise. The
    people come by band:
    - band 0: troglodytes (existing)
    - band 1: `Species::OlmFolk` (water-walkers)
    - band 2: `Species::BatRider` with a blowgun (ranged, uses `clear_shot`)
    They spawn at the cavern floor farthest from dwarves and target the breach. "A noisy fort
    draws raids that a quiet one avoids" is literally this probability.
  - **Endless life (DIG-13):** replace one-shot `populate_caverns` with `cavern_restock()`
    (seasonal). While a band has an "open edge" (cavern floor tiles touching the map edge:
    computed once at generation and stored in `World.cavern_edges: Vec<Pos>`, serde default,
    recomputed on load), top up to N creatures from the edge.
  - **Sounds (DIG-09):** events in unrevealed or unlit cavern tiles (a creature death, a fight,
    a raid assembling) emit an announcement "Screams echo from the lower caverns" instead of the
    full report. It adds to the overseer's `Sense::Danger` and to dwarves' fear (anxious ones get
    a thought).
  - **Webs (DIG-14):** a `Tile.web` bit (or `Game.webs: HashSet<Pos>`, sparse; prefer the set to
    avoid a Tile field). A spider spits a web at a target in range 5: the target is stuck for
    50-200 ticks. Webs on cavern ceilings and floors snare walkers. A web can be cut, or burned
    (B13 fire).
  - **Cave fliers (DIG-15):** `Species::CaveSwift` swarms (a new `Mover::Flier` that passes Open
    tiles with no floor within caverns). Mostly harmless. Bats over chasms.
  - **Gradual reveal (DIG-17):** `flood_reveal` on breach currently reveals the whole cavern.
    Change it to reveal only tiles within light radius or 8 tiles of a dwarf (incrementally as
    dwarves walk: `reveal_near(pos, r)` every 20 ticks for dwarves in caverns). `order_station`
    and `order_kill` refuse unrevealed targets.
  - **Dark caverns (DIG-08)** come from B13.
- **Steps:**
  1. Noise and irritation, and noise-driven raids by band. Test, the "Done when":
     `a_noisy_fort_draws_cavern_raids_a_quiet_one_avoids` (two games with the same seed:
     simulated noise vs none over 4 seasons).
  2. Endless cavern life from open edges. Test: `a_cavern_with_an_open_edge_restocks`.
  3. Sounds from the deep. Test: `a_fight_in_an_unseen_cavern_is_heard_not_seen`.
  4. Webs. Test: `a_spider_webs_a_dwarf_who_is_stuck_until_freed`.
  5. Cave fliers. Test: `swifts_fly_over_the_chasm`.
  6. Gradual reveal. Test: `breaking_into_a_cavern_reveals_only_what_is_near`.
- **Overseer & screen:**
  - The overseer learns "the deep is listening". A Cautious one avoids felling cavern trees when
    noise is high. A trouble `Trouble::CavernRaid` makes it wall off or door the breach (existing
    breach doors).
  - The brain gets a "Rumbling" sense (noise level).
  - On screen: a noise gauge on the status screen ("The caverns are restless"), and sound
    announcements in a distinct colour.
- **Measure:** cavern-raid sieges recorded with `SiegeNote.foe` from the caverns. Wildlife and
  cavern deaths ≤ baseline +30%. **Performance:** `cavern_floors()` must be cached
  (`Game.cavern_floor_cache` keyed on `dig_version`), because it is a full scan today.
- **Risks / open questions:**
  - Gradual reveal changes `flood_reveal` semantics, and existing tests
    (`digging_beside_the_sky_does_not_walk_the_whole_sky`) assert counts. Keep `flood_reveal`
    for the surface, and use the new behaviour for cavern bands only.
- **Parallel work:**
  - **Writes:** new `src/game/caverns.rs` (noise, raids, restock, sounds, webs, reveal);
    `creature.rs` +4 species and `Mover::Flier`; `world.rs: neighbors` (flier branch, guarded),
    `World.cavern_edges`; `combat.rs: populate_caverns` (call restock), `cavern_floors` (cache);
    the dig job in game.rs (`emit(Ev::Noise)`: 1 line).
  - **Reads / depends on:** F-EV `Ev::Noise`; F-LIGHT (unseen = unlit); B13 step 3 for "seen".
  - **Hot spots:** world.rs `neighbors` (a second guarded branch after W26's and B06's; land in
    sequence or agree on one `match m {}` block, see hot-spot refactors), creature.rs Species
    enum and `ALL` (append).
  - **Agent size:** one agent, 3 sessions.

### D02 Digging too deep  (effort: 4 sessions; risk: high)
- **Design:**
  - **Deep metal (DIG-05):** new `Mat::Adamant` (an invented name such as "starsilver" is better,
    following CLAUDE.md "invented names"). It is generated in terrain.rs as hollow vertical
    tubes (a ring of ore around open air) between the magma sea and the deepest cavern. It is
    worth ×20, and is a weapon and armour material (×2 armour, ×1.8 weapon).
  - **The underworld (DIG-04):** below the bedrock under the tubes: a sealed layer
    (`World.underworld: Option<(i32, i32)>`, a z-range; the map gets 3 extra z on generation,
    for new worlds only; old saves have none). Mining out more than N tiles of the ore (a tracked
    `Game.deep_mined`) or breaking into the layer opens it. A demon horde (`Species::Demon`
    variants, generated like `make_cave_horror`: names from syllables, powers from B22 if
    present) pours up via the tubes. A permanent `Game.underworld_open = true` keeps spawning
    from the breach every season until it is sealed (built walls over the tube top).
  - **Warnings (DIG-02):** as `deep_mined` approaches N, at 50%, 75% and 90%: tremors (a shake
    and a mild F05 quake), warm draughts, cavern life fleeing upwards (D01 restock is reversed:
    creatures move toward upper bands), and nightmares (a thought for dwarves sleeping below a
    z). After the breach, cavern bands get a "changed" flag: darker, with an ash floor tint.
  - **History (DIG-28):** in overworld history generation, some dwarf civ sites "dug too deep":
    a ruined site with a demon figure that roams (a megabeast-like figure with `kind: "demon"`).
    These figures can appear via `maybe_megabeast`. A fort near such a site starts with a rumour.
  - **Stranger layers (DIG-27):** each cavern band gets a theme from a small table:
    crystal caves (light-emitting gems for B13), fungal jungle (D03), and the drowned deep
    (DIG-26 lakes). The theme sets fauna and the floor material. Deeper bands draw stranger
    themes.
- **Steps:**
  1. Deep metal tubes, mining and value. Test: `the_deep_tubes_hold_starsilver`.
  2. Warnings by depth mined. Test: `mining_toward_the_limit_brings_tremors_and_nightmares`.
  3. The underworld and its breach. Test, the "Done when":
     `a_delver_that_mines_out_the_tube_opens_the_way_for_demons`.
  4. Sealing the breach, and the permanent change. Test: `a_walled_tube_stops_the_demons`.
  5. Too deep in history. Test: `a_world_remembers_a_fort_that_dug_too_deep`.
  6. Cavern themes. Test: `deep_bands_draw_stranger_themes`.
- **Overseer & screen:**
  - The **Greedy** trait and **Delve** ambition are tempted. `Sense::DeepMetal` (tubes found)
    boosts Delve and Prosper. A Cautious overseer stops at the 75% warning and notes it ("We will
    not dig deeper"). A Greedy one ignores warnings. This is the classic story.
  - Spotlight cards for every warning and for the breach. The chronicle titles the year "the Year
    the Deep Opened". A game-over page from the underworld names it.
- **Measure:** in a Delver-heavy batch (`--style delver`), ≥30% of forts reach the tubes; of
  those, the Greedy ones breach more often than the Cautious (logged in the trial).
- **Risks / open questions:**
  - A map-depth change for the underworld: generate only for new worlds. Older saves get the
    deep metal without the underworld.
  - **Balance:** demons will end forts. That's intended ("losing is fun"), but make sealing
    possible.
- **Parallel work:**
  - **Writes:** new `src/game/deep.rs`; `terrain.rs: generate_region` (tubes, underworld layer,
    themes; a new section, best in a new `terrain_deep.rs` called from it); `material.rs` +1 Mat;
    `overworld.rs` history (one event kind); `creature.rs` +1 species; `ambitions.rs` +1 sense.
  - **Reads / depends on:** D01 (bands, restock reversal), F05 (`quake`), B13 (themes' light),
    B22 (M4, for powers, optional).
  - **Hot spots:** terrain.rs `generate_region` (a single call line to the new module),
    material.rs Mat enum (append).
  - **Agent size:** one agent, 4 sessions. Step 5 (overworld) is separable.

### D03 Farms and forests below  (effort: 3 sessions; risk: low)
- **Design:**
  - **Rich cavern soil (DIG-10 = AGR-03):** B12 adds soil depth. Here, cavern floor materials
    (Loam and Clay in bands) give fertility ×1.5, and surface soil above deep rock ×0.7.
  - **Fungus forests (DIG-18 = AGR-22):** cavern `Tree` tiles become typed:
    `Tile.mat = Mat::Wood` stays, but a new `Mat::Towercap` gives `Log` of towercap (new Mat) when
    felled. Spores: each season, open dug soil tiles (Loam or Clay floor, `dug`) in or touching a
    cavern band with a towercap within 6 tiles grow a sapling (`TileKind::Shrub`, then a `Tree`
    after a season, i.e. the `regrow_shrubs` pattern).
  - **Chasm disposal (DIG-19):** a `Dump` zone over a chasm (CHASM feature) or a cavern drop
    deletes items and gives `Ev::Noise` (D01). Corpses dropped there feed cavern life (the
    restock bonus) and draw a horror more often.
  - **Underground waters (DIG-26):** cavern lakes (bigger pools in the band theme "drowned
    deep"), a waterfall where the UNDER_RIVER meets a band, and a raft:
    `BuildingKind::Raft` on water (a 1-tile floor that is `has_floor`; a dwarf poles it a tile
    per N ticks).
  - **Magma dwellers (DIG-20)** come from W26.
- **Steps:**
  1. Towercap wood and felling in the caverns. Test, the "Done when":
     `a_fort_with_no_surface_trees_logs_a_fungus_forest` (a flat_patch with surface trees
     removed).
  2. Spores spread into dug soil. Test: `towercap_spores_take_in_dug_cavern_soil`.
  3. Cavern soil fertility (with B12). Test: `cavern_loam_out_yields_surface_loam`.
  4. Chasm disposal. Test: `refuse_thrown_into_the_chasm_is_gone_and_heard_below`.
  5. Lakes, waterfalls and rafts. Test: `a_raft_crosses_a_cavern_lake`.
- **Overseer & screen:**
  - `Lack::Wood` (it exists, in troubles.rs) gets a new answer: designate chopping in the
    nearest revealed cavern band (it currently designates only around the embark), weighed
    against D01 noise. A Cautious overseer sends a squad escort (A04 patrol) with the
    woodcutters.
  - Delve gets a step "Farm the caverns" (a farm zone on cavern loam behind a door).
- **Measure:** ≥50% of forts that open a cavern log it within a year. Desert and glacier forts
  survive their first winter more often (fuel and wood).
- **Risks / open questions:** pulling dwarves into caverns raises deaths, and that is the point.
  Watch that Lack::Wood doesn't send lone woodcutters into raids.
- **Parallel work:**
  - **Writes:** new `src/game/cavern_farms.rs`; `material.rs` +1 Mat (Towercap); terrain.rs (tree
    material in bands, lake theme hooks); `troubles.rs` (`Lack::Wood` answer: one branch);
    `buildings.rs` +Raft; `farming.rs` (fertility read).
  - **Reads / depends on:** B12 `fertility(p) -> f32`; D01 `Ev::Noise`, restock; D02 themes
    (optional).
  - **Hot spots:** troubles.rs (one match arm).
  - **Agent size:** one agent, 3 sessions.

### D04 Hold up the roof  (effort: 4 sessions; risk: med)
- **Design:**
  - **Supports (ENG-24):** a `Support` building (a 1-tile stone or wood pillar, can be linked to
    a lever to pull it). `unsupported(q, r)` (disasters.rs) treats a built support and any
    `built_from` wall as rock. That is half true today (walls already count, because they are
    solid). Generalise collapse to *connectivity*: when a support is pulled or a tile dug,
    `check_ceiling` also checks whether the rock above lost all vertical support in a bounded
    region (a flood fill of solid tiles at z+1 that touch no solid below, ≤500 tiles). If so,
    that whole chunk falls (rubble, a crush per HLT-12 in D06).
  - Trees and towers topple onto what's below.
  - **Sabotage:** a lever-pulled support is a trap that drops a ceiling on a hall full of
    goblins: the overseer uses it in A06.
  - **Size-limited passages (ENG-22):** `SpeciesInfo.scale ≥ 2.0` (trolls, megabeasts) can't
    pass a 1-wide door tile without battering it. This is a guarded check in `World::passable`
    via a new `Mover::Large`. Cramped tunnels (1-wide) give a −hit penalty to crossbows.
  - **Secret doors and escape tunnels (ENG-23):** `Building.secret: bool`. Hostiles' paths treat
    a secret door as a wall unless they've seen it used (`siege_memory`). The overseer digs an
    escape tunnel from the shelter to a far surface exit (layout), used by A06's evacuation.
  - **Wear (ENG-51):** constructed walls and engravings older than N years lose `smooth` level
    slowly. Loose bricks: a 1-in-1000 yearly chance that an old wall becomes passable
    (`F-HP` damage) → an escape route for prisoners or an entry for vermin (B20).
  - **Moving sections (ENG-30):** `Lift` (a platform moving between z-levels on power, W26) and
    a crushing ceiling (a lever-dropped `Wall` with `built_from` over a corridor; it kills like
    a cave-in).
- **Steps:** 1 support buildings and connectivity collapse (test, the "Done when":
  `a_support_lets_a_wide_hall_stand_that_would_otherwise_fall`) · 2 lever-pulled supports as a
  trap (test: `pulling_the_support_drops_the_ceiling_on_the_goblins`) · 3 size-limited doors
  (test: `a_troll_must_batter_a_narrow_door`) · 4 secret doors and escape tunnels (test:
  `goblins_walk_past_a_secret_door`) · 5 wear and loose bricks (test:
  `an_old_wall_wears_and_a_brick_comes_loose`) · 6 lifts and crushers (test:
  `a_powered_lift_carries_a_dwarf_up_a_level`).
- **Overseer & screen:**
  - The O02/O03 layouts place supports in wide halls instead of rock pillars (the pillared
    temperament). The creak warning (`check_ceiling`) becomes a trouble the overseer answers with
    a support.
  - A Warlord plans a collapse trap in the trap corridor.
  - On screen: a support glyph, and a "collapse risk" overlay (tiles within 1 of the span limit).
- **Measure:** cave-in deaths ≤2% of all deaths. Halls wider than 9 appear in Architect forts.
- **Risks / open questions:**
  - Connectivity collapse can be expensive. Bound the flood fill, and run it only on dig and
    support change.
  - `Mover::Large` needs a 4th or 5th Mover. Keep the `match` in `passable` cheap.
- **Parallel work:**
  - **Writes:** `disasters.rs` (`unsupported`, `check_ceiling`, `cave_in` → a chunk fall); new
    `src/game/supports.rs` (supports, sabotage, secret doors, wear, lifts); `world.rs: passable`
    (`Mover::Large`); `buildings.rs` +3 kinds; `Building.secret`.
  - **Reads / depends on:** F05 (same file: land after), F-HP, B06 `siege_memory` (secret doors),
    W26 `power.rs` (lifts: do step 6 after W26a).
  - **Hot spots:** world.rs `passable` (also touched by D01 and W26 for movers: see the
    hot-spot refactor "mover table").
  - **Agent size:** one agent, 4 sessions.

### D05 Sickness in the halls  (effort: 5 sessions; risk: med)
- **Design:**
  - New `src/game/disease.rs`:
    `pub struct Ailment { kind: AilmentKind, stage: u8, since: u64, contagious: bool }`,
    `Dwarf.ailments: Vec<Ailment>` (serde default),
    `AilmentKind::{Fever, Flux /*vermin plague*/, Rot /*wound*/, Venom(VenomEffect), Wasting, Parasite}`.
    `disease_tick()` runs every 100 ticks in `step_periodic` (not per dwarf per tick).
  - **Contagion (HLT-06):** two dwarves sharing a room (`room_at`), dormitory or dining hall that
    day, one of them contagious → p × (1 − tough/2000). The hospital isolates (contagion is zeroed
    in Hospital zones).
  - **Origins:** swamp sites, stagnant water (W26), rotting meat (existing rot), and cursed
    objects.
  - **Vermin (HLT-25):** B20 (M4) vermin in stores carry `Flux`. Cats reduce it. If B20 isn't
    present, a simple per-stockpile vermin counter that grows with food count and shrinks per cat.
  - **Retching (HLT-11):** a squeamish dwarf (anxiety > 25) seeing a corpse or miasma for the
    first time drops their task for 20 ticks and leaves a `Vomit` mess (HLT-31).
  - **Hygiene (HLT-30):** `Dwarf.grime: u8` rises with work and blood. Washing: a new
    `Task::Wash` at a well or river with soap (soap already exists for wounds). Baths are a
    `Bathtub` building (gold for nobles). Clothes are cleaned (a clothier job).
  - **Blood and grime (HLT-31):** the sparse map `Game.stains: HashMap<Pos, Stain { blood: u8, mud: u8, vomit: u8 }>`
    (serde default, capped at 20k entries). Wounds drop blood, walkers track it (10% per step to
    the next tile), rain washes outside tiles, and a cleaning job mops (the existing hauling
    labor). Stains raise grime and infection chance.
  - **Poisons (HLT-26):** `SpeciesInfo.venom: Option<VenomEffect::{Paralysis, Swelling, Fever, Necrosis}>`
    for spiders, vipers and scorpions (all exist). It applies on a wound in `creature_attack`.
    Cures: antivenom (HLT-37) or cutting it out (the doctor).
  - **Herbal remedies (HLT-37):** an `Apothecary` workshop: plants → `Remedy` (new ItemKind) by
    kind. Used by `do_treat` (medicine.rs).
  - **Strange afflictions (HLT-35):** caught in evil biomes (blood rain) or from cursed items:
    wasting, parasites, and a rare transformation (the dwarf becomes a husk at death:
    `Species::Husk` exists).
- **Steps:** 1 the ailment model and contagion in shared rooms, plus hospital isolation (test,
  the "Done when": `a_sickness_spreads_through_the_dining_hall_until_quarantined`) · 2 vermin
  plague (test: `vermin_in_the_food_store_bring_the_flux_and_cats_help`) · 3 retching (test:
  `a_squeamish_dwarf_retches_at_the_first_corpse`) · 4 stains, grime and washing (test:
  `blood_is_tracked_through_the_hall_and_rain_washes_it`) · 5 baths and hygiene thoughts (test:
  `a_noble_enjoys_a_gold_bathtub`) · 6 venom (test: `a_viper_bite_swells_and_an_antivenom_cures_it`) ·
  7 apothecary remedies (test: `the_apothecary_makes_remedies_from_herbs`) · 8 strange
  afflictions (test: `a_dwarf_caught_in_blood_rain_can_waste`).
- **Overseer & screen:**
  - A trouble `Trouble::Sickness(n)`: the overseer quarantines, which means `Burrow` restrict
    the sick to the hospital (burrows.rs exists; the alert burrow code shows how). It builds an
    apothecary when sickness recurs, and assigns cats to food stores.
  - On screen: an ailment line on the dwarf sheet, a "Sick" filter in the dwarf list, a stains
    render (dim red floor), and an announcement "A fever is spreading".
- **Measure:** disease deaths 2-8% (present but not dominant). A quarantined outbreak ends
  within a season ≥70% of the time.
- **Risks / open questions:**
  - Stains map size and the per-step tracking cost: track only on dwarf moves into stained tiles,
    with a cap.
  - Contagion checks are O(dwarves²) per room: group by room once per check.
- **Parallel work:**
  - **Writes:** new `src/game/disease.rs`, new `src/game/hygiene.rs` (stains, grime, wash,
    baths); `dwarf.rs` +2 fields (`ailments`, `grime`) + `Task::Wash`; `creature.rs`
    `SpeciesInfo.venom`; `combat.rs: creature_attack` (venom: 2 lines); `medicine.rs: do_treat`
    (remedy branch); `buildings.rs` +2 (Apothecary, Bathtub); `items.rs` +1 (Remedy).
  - **Reads / depends on:** F-STEP; B20 (optional); W26 stagnant water (optional hook).
  - **Hot spots:** combat.rs `creature_attack` (2 lines), medicine.rs (one match arm).
  - **Agent size:** too big for one session. **D05a** (steps 1-3, disease core) and **D05b**
    (steps 4-8, grime, venom, remedies), parallel after the `Ailment` type lands.

### D06 Harsh world  (effort: 3 sessions; risk: low)
- **Design:**
  - **Sun sickness (HLT-05):** `Dwarf.below_since: u64`, the tick they were last outside. If
    they've been under for more than a year, the first trips outside give nausea (−work, retching
    via D05), which fades with exposure (`sun_tolerance: u8`). The "Done when".
  - **Surviving cave-ins (HLT-12):** `cave_in` damage scales with fall height and chunk size
    (today it is a fixed 7). Some are trapped: buried, with a `Task::Trapped` until dug out (a
    dig designation on the rubble tile).
  - **Burning dwarves react (HLT-13):** a dwarf on a `fire > 0` tile flees fire (`do_flee`
    variant, with fire tiles as threats) toward water within 10, and is extinguished when in
    water ≥1.
  - **Falls that hurt (HLT-23):** `update_dwarf`'s falling branch accumulates `Dwarf.falling: u8`.
    On landing, a wound by height (3 + 4 per level beyond 1). It catches on stair levels (stairs
    stop the fall). Death cause "fell to their death" goes in `annals::cause_of`.
  - **Cold injuries (HLT-22):** a `chill: u8` per dwarf, raised outside in Blizzard or Snow
    without clothes (clothing.rs knows what's worn), lowered indoors near a hearth (B13) or
    warm water. Thresholds bring shivering (thought), then frostbite (a hand or foot part wound),
    then death "froze to death". Dwarves shelter in extreme weather (the overseer's
    civilian alert on a blizzard).
  - **Blind in the dark (HLT-36):** from B13's `light_at`. In darkness, movement +50% cost (a
    `move_cd` multiplier), work −50%, and combat −hit.
- **Steps:**
  1. Sun sickness. Test, the "Done when": `a_dwarf_raised_below_falters_in_the_sun_then_adapts`.
  2. Falls that hurt, and the cause. Test: `a_dwarf_who_falls_three_levels_is_hurt`.
  3. Cave-in survival and trapping. Test: `a_dwarf_trapped_under_rubble_is_dug_out`.
  4. Burning dwarves seek water. Test: `a_burning_dwarf_runs_for_the_pond`.
  5. Cold and frostbite. Test: `a_dwarf_caught_in_a_blizzard_without_clothes_gets_frostbite`.
  6. Blind in the dark. Test: `a_miner_in_the_dark_walks_slower`.
- **Overseer & screen:**
  - A trouble `Trouble::Cold` (frostbite cases): the overseer orders clothes (clothing work
    orders exist) and builds hearths.
  - The civilian alert goes up on blizzards for Cautious overseers.
  - Thoughts and announcements, plus death causes in the report.
- **Measure:** new causes visible, each ≤5% of deaths. Falls are recorded (they are 0% today
  only because falls are harmless).
- **Risks / open questions:**
  - Falls will kill dwarves on ramps or stairs in old layouts, so check `settle` and `walkable`
    edge cases (a dwarf "falling" one level through a dug stair top).
  - Cold depends on clothing coverage, which exists.
- **Parallel work:**
  - **Writes:** new `src/game/harsh.rs` (sun, cold, trapped, burning); `game.rs: update_dwarf`
    (the falling branch and a 1-line `self.harsh_tick(d)` every 10 ticks); `disasters.rs:
    cave_in` (scaled damage); `dwarf.rs` +4 fields; `annals.rs: cause_of` (+2 causes);
    `combat.rs: do_flee` (accepts fire tiles via a `flee_from(&[Pos])` refactor).
  - **Reads / depends on:** F-LIGHT (step 6), B13 hearth (step 5 warmth; optional), D04 chunk
    collapse (step 3), D05 retching (step 1; optional).
  - **Hot spots:** game.rs `update_dwarf` (shared with F05's flee-water: land F05 first),
    disasters.rs `cave_in` (after D04).
  - **Agent size:** one agent, 3 sessions.

### D07 Marked for life  (effort: 4 sessions; risk: low)
- **Design:**
  - `Body.scars: Vec<Scar { part: Part, kind: ScarKind::{Cut, Burn, Bite, Brand, LostTeeth, LostEye}, from: String, tick: u64 }>`
    (serde default). Created when a `Gash` or worse heals (body.rs `heal`), or when
    `Ev::Wounded` reaches grade ≥ Mangle. Others notice them and nicknames come from them
    (C03 `SOC-10` if present: "Scarface"-style original words).
  - **Pain (HLT-18):** `Body.pain()` sums wound grades by part size, fading as wounds heal. It
    feeds `Fighter` (−hit) and fear (F-NERVE). Tolerance comes from `attrs.will`; an infection
    keeps it high.
  - **Prosthetics (HLT-16):** new `ItemKind::{PegLeg, Hook, IronHand}`, made at the Mechanic or
    Forge. A doctor fits them to a `lost` part (medicine.rs `Treatment::Fit`). They restore
    `can_hold`/walk partially; a skilled mechanic makes better ones (quality → restoration).
  - **Old wounds (HLT-14):** migrants (game.rs `migrants`) and history veterans arrive with
    scars or a lost part (10%), plus a grudge thought against the species.
  - **Head injuries (HLT-15):** a head Mangle can shift `Personality` facets ±20 (the temper
    flip), and notes it on the sheet.
  - **Old age (HLT-17):** `age` exists. From 120, attributes (W20) decay yearly (str and agi
    −3%), so old veterans miss more.
  - **Beards (HLT-34):** `Dwarf.beard: Beard { len: u8, style: u8, grey: bool }`. It grows
    yearly and greys with age. Singed by fire (B13 / D06 burning), shaved as a punishment
    (justice.rs) or a prank (drama.rs pranks), with shame thoughts. Purely flavour, cheap.
- **Steps:** 1 scars and the sheet (test: `a_healed_gash_leaves_a_named_scar`) · 2 pain (test:
  `a_dwarf_in_great_pain_fights_worse`) · 3 prosthetics (test, the "Done when":
  `a_veteran_with_an_iron_hand_can_hold_a_weapon_again`) · 4 old wounds on migrants (test:
  `some_migrants_arrive_scarred`) · 5 head injuries (test: `a_head_wound_can_change_a_temper`) ·
  6 old age decline (test: `old_dwarves_lose_strength`) · 7 beards (test:
  `a_beard_is_singed_by_fire_and_shaved_as_punishment`).
- **Overseer & screen:**
  - The overseer orders prosthetics when a soldier has a lost part (a new work order via `boosts`).
  - The sheet shows scars, beard, and prostheses. Chronicle mentions ("lost an eye at the Siege
    of …"). The O06 life story uses scars.
- **Measure:** ≥1 prosthetic fitted per 3-year Warlord fort. Pain changes the S07 bleed balance
  little (arena check).
- **Risks / open questions:** pain and fear may make soldiers worse; tune with W20 and A02 in the
  arena.
- **Parallel work:**
  - **Writes:** `body.rs` (Scar, pain, prosthesis effect in `can_hold` and `limp`); new
    `src/game/scars.rs` (scar creation subscriber, beards, old wounds, head injuries);
    `medicine.rs` (fit prosthesis); `items.rs` +3 kinds; `combat.rs: dwarf_fighter` (pain: 1
    line); `dwarf.rs` +1 field (beard).
  - **Reads / depends on:** F-EV `Ev::Wounded`; W20 (old age, will).
  - **Hot spots:** body.rs (owned by this item in its wave), dwarf.rs (append).
  - **Agent size:** one agent, 4 sessions.

### D08 The rest of the workshop row  (effort: 5 sessions; risk: low)
- **Design:**
  - Mostly data in `buildings.rs` (`BuildingKind`, `Recipe`) and `material.rs`.
  - **Melting (CRF-10):** a `Recipe::Melt` at the Smelter or MagmaSmelter on any metal item marked
    `melt: bool` (a new `Item` field, serde default, and an `m` key in the look menu). It returns
    `Bar` × weight/3. Melting a masterwork upsets its maker (D10 MND-35).
  - **More metals and gems (CRF-44):**
    - new Mats: Silver, Tin, Zinc, Brass, Platinum (Native…), and 4 gems (invented or
      generic names).
    - Ore placement in terrain.rs (veins by layer).
    - Alloys: bronze becomes copper + tin (today bronze is a direct recipe; check smelter
      recipes), brass is copper + zinc.
  - **Whole-carcass butchery (CRF-25):** `Butcher` outputs add skull, horn, teeth, fat, chitin
    and shell (new `ItemKind::BodyPart` with `label`), by species. A skull totem is a craft
    recipe.
  - **Leather (CRF-31):** hides by creature size: `scale` → number of leathers (1-4).
  - **Kiln, ceramics, lime (CRF-45):** a `Kiln` (clay → earthenware jugs and pots as containers,
    lime from limestone → `Plaster` for D04 walls).
  - **Glass (CRF-46):** a `GlassFurnace` (sand bags → glass goods and windows for B13).
  - **Oil, potash, lye (CRF-32):** a `Screwpress` (seeds and nuts → oil for B13 lamps),
    ash → potash (B12 fertiliser) and lye (soap: currently ash and tallow).
  - **Instruments (CRF-15):** a `Craft` variant `Instrument` played in the tavern (festivals.rs
    and the tavern use them: +party need).
  - **Toys (CRF-14):** `Toy` crafts for children (D10 play).
- **Steps:** 1 melting (test, the "Done when": `the_overseer_melts_captured_goblin_weapons_into_bars`) ·
  2 new metals, ores and alloys (test: `tin_and_copper_make_bronze`) · 3 butchery parts and
  leather by size (test: `a_bear_gives_four_hides_skull_and_fat`) · 4 kiln and lime (test:
  `the_kiln_fires_clay_into_jugs`) · 5 glass (test: `sand_becomes_green_glass_windows`) · 6 oil,
  potash and lye (test: `the_screwpress_makes_lamp_oil`) · 7 instruments and toys (test:
  `a_tavern_plays_instruments_and_children_get_toys`).
- **Overseer & screen:**
  - `ov_orders` (and O08's "orders that think", if landed) marks captured weapons for melting
    when the fort is short of bars. It builds a kiln when clay exists and barrels run short.
  - The UI build menu gets the new buildings (the two-column popup handles more). The look menu
    gets the melt key.
- **Measure:** bars per year +10% in Warlord forts (melting siege loot). There are no new
  shortages (`Lack` counts in the trial).
- **Risks / open questions:**
  - Changing bronze to an alloy recipe could starve existing forts of bronze. Keep the old direct
    recipe for one version.
  - Many new Mats: `Mat` enum serde names must be stable (append only).
- **Parallel work:**
  - **Writes:** `buildings.rs` (`BuildingKind` +5, `Recipe` +~15, the recipe table), `material.rs`
    (+9 Mats, props), `items.rs` (+4 kinds, `Item.melt`), `terrain.rs` (ore veins: one fn),
    `industry_tests.rs` (tests), `overseer.rs: ov_orders` (melt rule: a few lines).
  - **Reads / depends on:** none (independent).
  - **Hot spots:** buildings.rs enum and `ALL` and the `Recipe` match (also A05, B13, W26, D04,
    D05, D09 add buildings). **This is the biggest merge magnet in M6.** See the hot-spot
    refactor "building table".
  - **Agent size:** one agent, 5 sessions, or 2 agents: **D08a** (steps 1-3, metal and
    butchery) and **D08b** (steps 4-7, kiln, glass, press, instruments) on disjoint recipes after
    the building-table refactor.

### B11 Logistics  (effort: 5 sessions; risk: med (performance))
- **Design:**
  - **Multi-item hauling (ECO-46):** `Task::Haul` gets `more: Vec<u32>` (serde default) for
    up to 4 small items of the same stockpile class picked up within 3 tiles of the first, or a
    whole bin. `search_haul` builds the batch after finding the first item (a second, cheap
    `find_item` limited to radius 3). Topping up containers of the same thing reuses the `boxes`
    map.
  - **Stockpile links (ECO-36):** `Zone.give_to: Vec<u32>`, `take_from: Vec<u32>`, and
    `Building.take_from` for workshops. `haulable` respects links, and the workshop item search
    (jobs) restricts to linked piles when set.
  - **Quality filters and tidying (ECO-37):** `StockFilter` gains `min_quality`, `max_wear` and
    a `meltable` flag (D08), plus a "consolidate" order that packs loose items into bins.
  - **Wheelbarrows (ECO-47):** `ItemKind::Wheelbarrow`, assigned to stone piles
    (`Zone.barrows: u8`). A hauler carrying a heavy item (boulder) with a barrow moves at normal
    speed (today: a speed penalty? Check `carry` weight in game.rs; if none, add a weight-based
    `move_cd` multiplier first, so barrows matter).
  - **Burrow logistics (ECO-48):** a burrow stock target, `Burrow.stock: Vec<(StockFilter, u16)>`;
    transfer jobs move goods between piles in different burrows.
  - **Minecarts (ECO-29):** a `TrackTile` construction (carved track: a Desig on floor), `Stop`
    buildings with conditions, and a `Minecart` creature-like entity with speed (`Game.carts: Vec<Cart>`)
    that moves along track each tick, picks up from a stop's linked pile and drops at the next.
    Downhill (ramps) accelerates, and corners at high speed derail (the cart flies, wounding
    anyone in the way). Rollers use W26 power.
- **Steps:** 1 multi-item hauling and bins (test, the "Done when" part 1:
  `a_hauler_carries_several_items_at_once`) · 2 stockpile links (test:
  `a_linked_pile_feeds_only_its_workshop`) · 3 quality filters and consolidate (test:
  `a_masterwork_pile_refuses_poor_goods`) · 4 wheelbarrows and carry weight (test:
  `a_wheelbarrow_moves_boulders_faster`) · 5 burrow stock targets (test:
  `a_burrow_pulls_food_to_its_stock_target`) · 6 track, stops and carts (test, part 2:
  `a_minecart_line_feeds_a_workshop`) · 7 derailment (test: `a_fast_cart_flies_off_a_corner`).
- **Overseer & screen:**
  - The overseer links piles beside workshops (its layout knows where workshops are: `ov_fit`),
    assigns barrows to the quarry pile, and Builder/Prosper forts lay a cart line from a deep
    mine to the smelter.
  - The hauling count in the F3 overlay. Carts render as moving glyphs.
- **Measure:**
  - Haul trips per item fall ≥30% (a counter in the trial).
  - **Performance:** `one_year_timing` should *improve* (fewer `search_haul` calls). The
    `SEARCHES` counter goes down.
  - Idle-dwarf time down.
- **Risks / open questions:**
  - `search_haul` is a hot, subtle function (reservations, containers). Refactor it into a
    `haul.rs` module first (pure motion), then add batching.
  - Minecarts are L on their own; they could slip to M7 if the milestone runs long. Decision for
    the user.
- **Parallel work:**
  - **Writes:** new `src/game/haul.rs` (moved `find_haul`, `search_haul`, haul execution);
    `dwarf.rs: Task::Haul` (+field); new `src/game/carts.rs`; `game.rs` (`Zone` fields,
    `StockFilter`, removal of the moved functions); `burrows.rs` (stock targets).
  - **Reads / depends on:** W26 `power.rs` (rollers only); D08 `Item.melt` (the filter flag:
    optional).
  - **Hot spots:** game.rs (the move-out refactor touches the big file once; do it in wave 0
    so nobody else edits `search_haul` concurrently; A01 adds one predicate to it, so land A01's
    predicate before the move or apply it in haul.rs after).
  - **Agent size:** too big for one session. **B11a** (steps 1-5) and **B11b** (steps 6-7,
    carts), sequential.

### D09 The barnyard  (effort: 5 sessions; risk: med)
- **Design:**
  - Animal needs and products in `animals.rs` and a new `src/game/husbandry.rs`.
  - **Grazing (AGR-11):** `SpeciesInfo.grazer: u8` (food per day). Pasture tiles hold
    `Game.grass: HashMap<Pos, u8>`, or reuse `Tile.mat == Grass` plus a regrowth counter in a
    sparse map. Daily, each grazer eats from its pasture: tiles go to dirt and regrow in spring
    and summer. Cave moss on cavern floors counts. Hunger: `Creature.hunger: u16` (serde
    default); starving animals sicken, then die.
  - **Eggs (AGR-12):** a `NestBox` building. Fowl (new Species: hen, plus cave fowl) lay `Egg`
    items that hatch if left. Dragon and roc nests in lairs (B03 loot).
  - **Wool, milk and cheese (AGR-16):** `Task::Milk`/`Shear` on adult female goats and sheep (a
    timer per animal, `Creature.product_at: u64`). A `Dairy` workshop: milk → cheese.
  - **Bees (AGR-13):** a `Hive` building colonised from a wild hive; honey and wax via the
    screwpress (D08); mead at the still.
  - **Herd management (AGR-25):** pasture rules: cap, cull the old males, geld (a flag that stops
    breeding). Replace lost handlers (owner reassignment when the owner dies: exists for dogs).
  - **Breeding needs contact (AGR-26):** today it is `near(o.pos, c.pos, 12)`. Require the same
    pasture and adjacency within the last day.
  - **Training knowledge (AGR-07):** `Game.taming: HashMap<Species, u16>` fort knowledge, which
    grows with tries. Training decays without practice: `Creature.trained: u8` drops, and
    half-tamed beasts can go wild (`tame = false`, hostile for a while). Only young ones train
    fully.
  - **Trainers (AGR-30):** `Labor::AnimalTraining`, plus training jobs at a pasture. An animal
    performs (war-dog attack bonus) only with its handler within 10 tiles.
  - **War beasts (AGR-17):** trained bears and big cave beasts as war animals (generalise
    `Species::Dog`'s owner-follow code in `tame_tick` to any `war: bool` creature), with
    armoured barding later.
- **Steps:** 1 grazing and animal hunger (test, the "Done when" part 1:
  `a_pasture_keeps_goats_fed_through_summer_and_an_overgrazed_one_starves`) · 2 milking,
  shearing and the dairy (test, part 2: `goats_are_milked_and_milk_becomes_cheese`) · 3 eggs and
  nest boxes (test: `hens_lay_and_left_eggs_hatch`) · 4 bees (test: `a_hive_gives_honey_and_wax`) ·
  5 herd rules and breeding contact (test: `apart_animals_dont_breed_and_gelded_ones_never`) ·
  6 training knowledge and decay (test: `untrained_beasts_go_wild`) · 7 trainers and cues (test:
  `a_war_dog_fights_better_near_its_handler`) · 8 war beasts (test: `a_trained_bear_follows_its_soldier`).
- **Overseer & screen:**
  - Provision ambition: the overseer zones pastures sized to the herd, builds nest boxes and a
    dairy, and culls by rule.
  - A trouble `Trouble::Overgrazed`.
  - On screen: the pasture look shows grass and herd count. Animal sheets show hunger and
    training.
- **Measure:** food per head from animal products ≥15% in Provision forts. Animal starvation is
  rare.
- **Risks / open questions:**
  - Grazing on every pasture tile daily is cheap (daily). But `Creature.hunger` adds a field to a
    hot struct; fine.
  - Species additions: hen and sheep in `fauna()`, or bring them on embark.
- **Parallel work:**
  - **Writes:** `animals.rs: tame_tick` (breeding contact, war generalisation), new
    `src/game/husbandry.rs` (grazing, products, bees, training); `creature.rs` (Creature +4
    fields, SpeciesInfo +grazer, +2 species); `buildings.rs` +3; `items.rs` +5 kinds (Egg, Milk,
    Cheese, Wool, Honey); `dwarf.rs: Task` +2; `labor.rs` +1.
  - **Reads / depends on:** D08 screwpress (bees' wax: optional).
  - **Hot spots:** creature.rs (append fields), the buildings and items enums.
  - **Agent size:** too big for one session. **D09a** (steps 1-4, products) and **D09b**
    (steps 5-8, herd and training), parallel after step 1's grazing type.

### B12 Living farms  (effort: 4 sessions; risk: med)
- **Design:**
  - A **fertility model** in a new `src/game/soil.rs`:
    `pub fn fertility(&self, p: Pos) -> f32 = soil_depth(p) * moisture(p) * nutrients(p) * skill`.
    It is used in `grow_farms` (growth step = `fertility` × 1, so dry fields grow slower) and in
    the harvest yield (`n` scaled).
  - **Soil depth (AGR-03):** derived. It is the distance from the soil tile down to rock (from
    terrain layers: `Tile.mat` Loam, Clay or Sand under the surface) plus the cavern bonus (D03).
    Computed once per farm zone and cached.
  - **Moisture (AGR-36):** sparse `Game.soil: HashMap<Pos, Soil { wet: u8, nutrients: u8, weeds: u8, fallow_since: u64 }>`
    (serde default) for farm tiles only. Wet rises with rain (weather), a bucket watering job
    (`Task::Water`), or water within 2 tiles (irrigation channels from W26). It dries daily, and
    drought dries faster.
  - **Fertiliser and fallow (AGR-37):** each harvest −nutrients. Potash (D08) or manure
    (D09 animals in pasture leave it: a counter) +nutrients. A fallow season restores. The farmer
    skill multiplier already exists in the yield.
  - **Flood farming (AGR-08):** F05's spring melt over farm tiles sets wet = max and nutrients
    +2 (silt).
  - **Weeds:** untended tiles (no farmer visit for 10 days) grow weeds, and growth slows by a
    weed factor. A `Task::Weed` job.
  - **Orchards and growths (AGR-21):** surface trees carry seasonal fruit (a `Game.growths` sparse
    map: the tree pos → fruit count), gathered via the existing `find_gather`/`do_gather` (with
    a stepladder item for height). Saplings can be planted (`Task::Plant` on a non-farm soil tile
    with a fruit seed).
  - **Underground wood (AGR-22):** the reference to D03 (towercaps planted in dug soil).
  - **Multi-tile trees (AGR-24):** a tree gets a size (`Game.big_trees: HashMap<Pos, TreeShape { height: u8, crown: u8 }>`).
    Felling drops it away from the cutter: tiles in a line take crush damage (a cave-in-style
    wound) and logs = height. The renderer draws a crown on z+1..h. This is the riskiest step
    (world tiles for branches), so offer it as optional.
- **Steps:** 1 the soil model, depth and the fertility hook in growth and yield (test:
  `deep_loam_outgrows_thin_soil`) · 2 moisture, watering and irrigation (test, the "Done when"
  part 1: `an_irrigated_field_yields_more_than_a_dry_one`) · 3 weeds (test, part 2:
  `weeds_take_an_untended_field`) · 4 fertiliser and fallow (test:
  `potash_restores_a_tired_field`) · 5 flood farming (test: `the_spring_flood_enriches_riverside_fields`) ·
  6 orchards and planting trees (test: `a_fruit_tree_bears_in_its_season`) · 7 multi-tile trees
  and felling (test: `a_felled_tree_falls_away_and_crushes_a_cart`) (optional).
- **Overseer & screen:**
  - Provision: farms next to water. `ov_fit` digs an irrigation channel from the cistern (W26)
    to underground farms. It assigns a weeding labor, and orders potash.
  - A trouble `Trouble::TiredSoil`.
  - On screen: the farm look shows wet, nutrients and weeds. A soil overlay.
- **Measure:** food per head in year 2 ≥ M5 (farming must not get *harder* overall without the
  overseer compensating). Starvation deaths ≤ M5.
- **Risks / open questions:**
  - Slower farms could re-starve forts. Make the baseline fertility of a neutral, watered tile
    1.0 (today's speed) and let only neglect go below.
  - Multi-tile trees are a big rendering change. Suggest deferring to "Later".
- **Parallel work:**
  - **Writes:** new `src/game/soil.rs`, new `src/game/orchards.rs`; `game.rs: grow_farms` (1-line
    multiplier), the harvest branch in `do_task` (yield multiplier: 1 line), `find_farm`
    (weeding and watering jobs); `farming.rs` (gather growths); `dwarf.rs: Task` +2.
  - **Reads / depends on:** W26 (irrigation: wet from nearby water works without W26 too), F05
    (flood), D08 (potash), D09 (manure), D03 (cavern soil).
  - **Hot spots:** game.rs `grow_farms` and the harvest arm (2 lines). Everything else is in new
    files.
  - **Agent size:** one agent, 4 sessions (without step 7).

### D10 Wants and pastimes  (effort: 4 sessions; risk: med (stress balance))
- **Design:**
  - **More needs (MND-24):** `NeedKind` grows by 8: Wander, Family, Admire, GoodFood, Create,
    Argue, Trinket, Animals. They are appended, because `needs: Vec<Need>` is by kind and old
    saves pad. Each has a satisfier hook:
    - Wander: walking outside or in a cavern.
    - Family: being within 10 tiles of kin, paused if kin are away on an expedition.
    - Admire: engravings exist (`smooth >= 2`, admired in passing today), plus displays (MND-36).
    - GoodFood: meal quality.
    - Create: craft.
    - Argue: drama.rs insults.
    - Trinket: owning a craft.
    - Animals: being near pets or pasture.
    `needs_tick` gets a table instead of a match.
  - **Wider preferences (MND-42):** `Dwarf.likes: Vec<Like::{Creature(Species), Plant(Mat), Dish(ItemKind, Mat), Drink(Mat)}>`
    (up to 3), with a capped happy thought on seeing a liked animal.
  - **Food that matters (MND-43):** a rolling `Dwarf.last_meals: [(ItemKind, Mat); 4]`. Eating the
    same thing all four times gives a −thought, and the complaint becomes a fort-wide trouble.
    Nobles are fussier.
  - **Hobbies (MND-47):** idle dwarves with `Create` or `Trinket` needs whittle (a bone or wood
    craft, low quality), knit, or collect pebbles: an `Idle` alternative in the idle branch.
  - **Hip flasks (MND-44):** `Dwarf.flask: u8` sips, refilled at the still or stores. They are
    sipped on long jobs and during sieges (the Drink need, which C08 the bottle may own:
    coordinate).
  - **Smoking circles (MND-45):** a smokeleaf crop (a new Mat in the Crop table), pipes (a
    craft), and gatherings of 3+ at the meeting zone: Socialize ×2.
  - **Toys and play (MND-46):** children want toys (D08). Adults fiddle with puzzle boxes.
  - **Displays (MND-36):** a `Pedestal` or `DisplayCase` building holding an item. Passers-by
    admire it like engravings (reuse the admire thought code that exists for engravings).
  - **Pride (MND-35):** an item records its maker (`Item.maker: Option<u32>`, serde default; may
    already exist for artifacts: check `label`). When one is traded, melted or destroyed, the
    maker gets a −thought, ×3 if it was their only masterwork.
  - **Life dreams (MND-10):**
    `Dwarf.dream: Option<Dream::{MasterCraft(Labor), Family, Masterwork, SlayBeast, SeeArtifact}>`.
    Reaching it gives a permanent stress floor raise ("lasting calm") and a chronicle line.
  - **Values and facets (MND-25):** `Personality` +6 facets (curiosity, greed, bravery, piety,
    humour, vanity) and `Values` from the civ's culture (craft, family, martial, nature). This is
    the largest step: facets feed thoughts (e.g. martial values like the military). Default to 0
    for old saves.
- **Steps:**
  1. A needs table and the first 4 new needs. Test, the "Done when":
     `a_dwarf_who_admires_art_and_never_sees_any_is_unhappier`.
  2. The other 4 needs. Test: `a_dwarf_whose_kin_are_away_does_not_pine`.
  3. Preferences and meal variety. Test: `the_same_meal_every_day_brings_complaints`.
  4. Displays and pride. Test: `melting_a_dwarfs_only_masterwork_upsets_them`.
  5. Hobbies, flasks, smoking and toys. Test: `idle_dwarves_whittle_and_smokers_gather`.
  6. Life dreams. Test: `reaching_a_life_dream_brings_lasting_calm`.
  7. Values and facets. Test: `a_martial_dwarf_is_happier_in_a_squad`.
- **Overseer & screen:**
  - `Sense::Unrest` already exists. New troubles: `Trouble::SameFood` (the overseer diversifies
    kitchen orders), `Trouble::NoArt` (it engraves or places displays: the Revel ambition gains a
    "displays" step).
  - Dreams surface as spotlight cards when fulfilled. The sheet gets a Needs tab with all 15 and
    the dream.
- **Measure:** the stress distribution in the batch stays within ±10% of M5's mean (new needs
  must not swamp the fort). Tantrum deaths ≤ M5.
- **Risks / open questions:**
  - Doubling the needs doubles unhappiness sources. Scale each need's decay rate so the total is
    near today's.
  - Overlap with C08 (the bottle) and C11 (table and tankard) from M3. Reuse their food and drink
    work.
- **Parallel work:**
  - **Writes:** `needs.rs` (the table), `dwarf.rs` (`NeedKind` +8 appended, `NEED_KINDS`, +5
    fields, `Personality` +6 fields with serde defaults); new `src/game/pastimes.rs` (hobbies,
    flasks, smoking, toys, displays); new `src/game/dreams.rs`; `psyche.rs: eat_thoughts`
    (variety: 3 lines); `buildings.rs` +2.
  - **Reads / depends on:** D08 (toys, instruments, item maker), F-EV (`Ev::Slain` for the
    SlayBeast dream).
  - **Hot spots:** dwarf.rs `NeedKind`/`Personality` (append only), needs.rs (owned by this
    item), `psyche.rs eat_thoughts`.
  - **Agent size:** too big for one session. **D10a** (steps 1-4) and **D10b** (steps 5-7),
    sequential (both touch needs.rs).

### D11 Omens and skies  (effort: 3 sessions; risk: low)
- **Design:**
  - **Sky events (WLD-03):** `Game.sky: Option<SkyEvent { kind: SkyKind::{Eclipse, BloodMoon, Conjunction, Comet}, until: u64 }>`,
    scheduled from the world seed (deterministic: an eclipse every few years, known to
    astronomers). Work stops for a short while. The superstitious (anxiety > 25) fret. The pious
    (C10's piety, or D10's piety facet) feast. B13 dims the sky light.
  - **Prophecies (WLD-08):** an engraving, or a tablet found in ruins (F08 ruins), foretells an
    upcoming scheduled event: a sky event, a beast's return (a megabeast already planned by
    `maybe_megabeast`; make its timing pre-rolled to allow foretelling), or a fall (a siege).
    `Game.prophecies: Vec<Prophecy { text, event: Foretold, due: u64, fulfilled: Option<bool> }>`.
    The fort can try to prevent it: e.g. arm up ("Muster" boosted), or seal a door.
  - **Thin ice (WLD-04):** `Tile.frozen` becomes `ice: u8` thickness (serde: map the old bool
    to 3). It grows daily in cold and thins at the edges of winter. A walker on ice <2 has a
    chance to break it: the dwarf drops into water (the existing drowning), and
    `scramble_ashore` exists.
  - **Wild weather (WLD-11):** `Weather::{Hail, DustStorm, Tornado, Hurricane}`, by biome. Hail
    hurts the outside (small wounds) and damages crops. A dust storm reduces sight (B13 light
    cap). A tornado is a moving creature-like cell that shoves items and dwarves 2-4 tiles (a
    fall: D06). Hurricanes are coastal: rain plus wind (windmills over-spin, W26).
  - **Deep snow and puddles (WLD-16):** `snow` becomes depth (a sparse map `Game.snow_depth`, or
    reuse the bool plus a map). Deep snow gives +move cost and lingers. Rain puddles: a hollow
    (a floor tile lower than its neighbours, from `surface_z`) gets `water = 1` in rain and dries
    via `dry_up`.
- **Steps:** 1 sky events (test, the "Done when": `an_eclipse_unsettles_the_fort_and_the_pious_feast`) ·
  2 prophecies (test: `a_ruin_tablet_foretells_the_eclipse_that_comes`) · 3 thin ice (test:
  `early_winter_ice_breaks_under_a_dwarf`) · 4 wild weather kinds (test:
  `hail_hurts_dwarves_outside_and_a_tornado_shoves`) · 5 deep snow and puddles (test:
  `deep_snow_slows_walkers_and_rain_fills_hollows`).
- **Overseer & screen:**
  - The **Pious** trait: a feast on an eclipse (festivals.rs). A Cautious overseer heeds
    prophecies (a `Sense::Foretold` boosting Fortify or Muster before a foretold siege).
  - Shelter during hail and tornado (the civilian alert).
  - On screen: the sky shown in the status bar (an eclipse darkens the map tint), prophecy text
    in the legends, and spotlight cards.
- **Measure:** weather deaths ≤3%. ≥1 prophecy fulfilled or averted per 10 forts.
- **Risks / open questions:**
  - `frozen: bool` → `ice: u8` touches `passable` and `fluids` (they check `t.frozen`). Keep
    `frozen` and add `ice: u8` beside it (serde default), so `frozen` stays the walkability flag.
  - Pre-rolling megabeast timing changes RNG streams, so batch numbers shift; compare carefully.
- **Parallel work:**
  - **Writes:** `weather.rs` (Weather +4, `weather_daily`, `seasonal_ice`, `dry_up`); new
    `src/game/omens.rs` (sky, prophecies); `world.rs: Tile` +1 (`ice`); `combat.rs:
    maybe_megabeast` (pre-roll); the tornado in `omens.rs`.
  - **Reads / depends on:** F-LIGHT (dimming: optional), F08 ruins (exists), D06 falls
    (tornado: optional).
  - **Hot spots:** world.rs `Tile` (append, `skip_serializing_if`), combat.rs `maybe_megabeast`
    (small).
  - **Agent size:** one agent, 3 sessions.

---

## Work packages and waves

Sessions are per package. Every package ends `cargo test --release` green. Batches are run by
the coordinator once per wave after merging.

### Wave 0: foundations and hot-spot refactors (serial or 2 agents; about 4 sessions)
| Package | Contents | Files | Notes |
|---|---|---|---|
| P0a | F-STEP: split `step`; F-EV: `events.rs` + `drain_events`; move `fluids()` out of game.rs to `water.rs`/`fluids.rs` | game.rs, new events.rs | Pure motion plus the queue. One agent, 1 session. **Must land first.** |
| P0b | Hot-spot refactors: `haul.rs` move-out (`find_haul`, `search_haul`, the haul arm); the building table refactor (below); the mover table (below) | game.rs, buildings.rs, world.rs | One agent, 1-2 sessions, after P0a (both touch game.rs). |
| P0c | F-NERVE, F-ATTR types only (fields + defaults + empty fns), F-CAPT, F-HP, F-LIGHT stubs, `siege_path.rs` skeleton with its pure tests | new files; dwarf.rs, creature.rs (1-3 appended fields each) | One agent, 1 session. Parallel with P0b (disjoint). |

### Wave 1 (parallel; no shared functions)
| Package | Item steps | Sessions | Conflicts with |
|---|---|---|---|
| P1 W20 | all | 2 | owns `strike` and `dwarf_fighter` this wave |
| P2 A01 | steps 1-4, 6 (step 5 waits for nothing: F-CAPT is in P0c) | 3 | `creature_think` siege branch (no one else in wave 1) |
| P3 B06a | B06 1-4 | 3 | `world.rs neighbors` via the mover table only |
| P4 F05 | all | 3 | `update_dwarf` drowning branch (no one else) |
| P5 D05a | D05 1-3 | 2 | none |
| P6 D07 | all | 4 | body.rs (owner); `dwarf_fighter` 1 line (merge after P1) |
| P7 D08a | D08 1-3 | 2-3 | building table rows (append-only after P0b) |
| P8 D11 | all | 3 | `Tile` append, `maybe_megabeast` |
| P9 B11a | B11 1-5 | 3 | haul.rs (owner) |

Serialize within wave 1: P1 before P6's `dwarf_fighter` line (or P6 rebases). Everything else can
run at the same time. Batch after wave 1: survival, siege kills (W20), drowning (F05),
disease (D05a).

### Wave 2
| Package | Item steps | Sessions | Needs |
|---|---|---|---|
| P10 A02a | A02 1-4 | 2 | P1 (fight maths), F-NERVE |
| P11 B13a | B13 1-4 | 2 | F-LIGHT |
| P12 W26a | W26 1-3 | 3 | P0a fluids move |
| P13 B06b + B06c | B06 5-9 (two agents) | 2 + 2 | P3 |
| P14 D05b | D05 4-8 | 3 | P5 |
| P15 D08b | D08 4-7 | 2-3 | P7 |
| P16 D09a | D09 1-4 | 2-3 | none (P7's screwpress optional) |
| P17 D10a | D10 1-4 | 2 | D08a (item maker) |

Serialize: P10 owns `combat_awareness` and `creature_tick` breaking; P13's B06c (feints) touches
`creature_think` only through `siege_ai.rs`. Batch after wave 2: routs, breaches, timing.

### Wave 3
| Package | Item steps | Sessions | Needs |
|---|---|---|---|
| P18 A02b | A02 5-9 | 3 | P10 |
| P19 A03 | all | 3 | F-EV, P10 (for nemesis fear) |
| P20 A04a | A04 1-4 | 2-3 | P10 |
| P21 A05 | 1-5 | 4 | P3 (B06a) |
| P22 B13b | B13 5-6 | 2 | P11 |
| P23 W26b | W26 4-7 | 2-3 | P12 |
| P24 D01 | all | 3 | P11 (seen), F-EV |
| P25 D04 | all | 4 | P4 (disasters.rs), P12 (lifts, step 6) |
| P26 D09b | D09 5-8 | 2 | P16 |
| P27 D10b | D10 5-7 | 2 | P17 |
| P28 B11b | B11 6-7 | 3 | P9, P12 (rollers) |

Serialize: P18 and P20 both edit `ov_alert`, so P18 lands first; P20 replaces the station loop
with alerts. P21 and P18 are disjoint.

### Wave 4
| Package | Item steps | Sessions | Needs |
|---|---|---|---|
| P29 A04b | A04 5-9 | 2-3 | P20 |
| P30 A06 | all | 3 | P18 (levy), P2 (floodgate), B02 (M4) |
| P31 B03a | 1-4 | 3 | P18, F-CAPT, B18 (M4) |
| P32 D06 | all | 3 | P11, P25 (cave-in), P22 (hearth) |
| P33 B12 | 1-6 | 4 | P23 (irrigation), P4 (flood), P15 (potash), P16 (manure) |

### Wave 5
| Package | Item steps | Sessions | Needs |
|---|---|---|---|
| P34 B03b | 5-8 | 3 | P31 |
| P35 D03 | all | 3 | P33 (fertility), P24 (noise, restock) |
| P36 D02 | all | 4 | P24, P4, P23 (magma creatures) |

The milestone order is preserved in the end: M5's last packages (P29-P31, P34) can overlap M6's
waves because they touch different files. Running them together cuts the calendar time to about
5 waves instead of about 9.

---

## Interface contracts (summary)

| Contract | Lives in | Signature | Serde |
|---|---|---|---|
| Event queue | `game/events.rs` | `enum Ev {…}`, `Game::emit(Ev)`, `drain_events()`, subscribers `fn on_event(&mut self, &Ev)` | `#[serde(skip)] events: Vec<Ev>` |
| Nerve | `game/morale.rs` | `struct Nerve { fear: i16, state: NerveState }`; `courage_of_dwarf`, `courage_of_creature`, `frighten(Who, i16, &str)` | `#[serde(default)] nerve` on Dwarf and Creature |
| Attributes | `dwarf.rs` + `game/attributes.rs` | `struct Attrs { str, agi, tough, endur, will, focus: u16 }`, `Fighter.{str,agi,tough}: f32`, `Dwarf.weapon_xp: [u32;6]` | default 1000; `fix_after_load` seeds `weapon_xp` |
| Captives | `game/captives.rs` | `struct Captive`, `enum Held`, `enum Holding`; `fn take_captive(&mut self, c: Creature, holding) -> u32`, `fn free_captive(id) -> Option<Creature>` | `#[serde(default)] captives` |
| Levy | `game/morale.rs` (A02b) | `fn levy(&mut self, reason: &str) -> usize`, `Game.martial_law: Option<u64>`, `Squad.levy: bool` | defaults |
| Floodgate weapon | `game/traps.rs` (A01) | `fn floodgate_trap(&self, lever: u32) -> FloodPlan { tiles: Vec<Pos>, hostiles: usize, dwarves: usize }` | n/a |
| Structure HP | `game/structures.rs` | `toughness(Pos) -> u16`, `batter(Pos, force: u16, by: &str) -> bool` | `#[serde(default)] damage: HashMap<Pos,u16>` |
| Siege planner | `src/siege_path.rs` | `plan(world, start, goal, &SiegeCaps, hp, limit) -> Option<(Pos, Vec<(Pos, Breach)>)>` | the plan cache is `serde(skip)` |
| Light | `game/light.rs` | `light_at(Pos) -> u8`, `seen(bool, Pos) -> bool`, `light_dirty()`, `is_night()` | `serde(skip)` cache |
| Power | `game/power.rs` | `struct Network { nodes, supply, demand }`, `powered(bld: u32) -> bool` | rebuilt on load |
| Fertility | `game/soil.rs` | `fertility(Pos) -> f32` (1.0 = today's speed) | `#[serde(default)] soil: HashMap<Pos, Soil>` |
| Ailments | `game/disease.rs` | `struct Ailment`, `infect(dwarf: u32, AilmentKind, why)` | `#[serde(default)] Dwarf.ailments` |
| Noise | `game/caverns.rs` | `Ev::Noise { pos, loud }` → `cavern_noise: [u32;3]` | default |
| Battles | `game/annals.rs` | `struct Battle {…}`, `Annals.battles` | default |

Enums that several items extend (`BuildingKind`, `Recipe`, `ItemKind`, `Mat`, `Species`,
`Task`, `NeedKind`, `Ambition`, `Sense`, `Trouble`/`Lack`): **append only**, at the end, each
addition in its own small commit at the start of its package. Merge conflicts in these become
trivial adjacent-line conflicts the coordinator resolves by keeping both.

---

## Hot-spot discipline (refactors before the waves)

1. **`Game::step` → per-system blocks** (F-STEP): one line per system. New systems never touch
   another system's line.
2. **`creature_died` → events:** keep only the core (drop gear, spawn a corpse, kill the figure in
   the overworld) and `emit(Ev::Slain{…})`. Move the fort-wide rejoicing, kill-order completion
   and rout check into subscribers (`victory.rs`, `morale.rs`). Today every M5 item would edit
   this 90-line function.
3. **`search_haul`/`find_haul` → `haul.rs`** (before B11 and before A01's jammed-trap predicate).
4. **The building table:** `BuildingKind::ALL`, `parts()`, size, name, glyph and recipes are
   spread over several `match`es in buildings.rs. Collapse them into one `const BUILDINGS: &[BuildingDef]`
   table (a name, glyph, size, parts, recipes and labor per row), so adding a building is one
   row. A05, B13, W26, D04, D05, D08, D09 and D10 all add buildings: about 25 new kinds across
   M5/M6.
5. **The mover table:** `World::passable`/`neighbors` gain movers (Climber B06, Large D04, Flier
   D01, Magma W26). Change `Mover` handling to `fn mover_rules(m) -> MoverRules { doors: bool, climb: bool, fly: bool, magma: bool, large: bool }`,
   a const lookup, with one extra-edges block guarded by `if rules.any_special()`. The common
   Dwarf, Beast and Goblin path stays as fast as today. Benchmark with `one_year_timing`.
6. **`needs_tick` → a table** of `(NeedKind, decay, satisfier fn)` before D10.
7. **`creature_think`'s siege branch → `siege_ai.rs`:** move it out of combat.rs with no
   behaviour change (A01 and B06 both edit it; after the move, A01 edits arrival code in
   combat.rs and B06 edits `siege_ai.rs`).
8. **UI panes:** new screens (combat log, routines grid, engines list, needs tab, overlays) go in
   new `ui_*.rs` modules called from `ui.rs` by one line each. ui.rs is 5131 lines and a guaranteed
   conflict source.

---

## Verification per package

Each package adds the tests named in its steps (in the module's `#[cfg(test)] mod tests`) and
passes `cargo test --release`. In addition:

| Package | Extra checks by the agent | Coordinator's batch after the wave |
|---|---|---|
| P0a/P0b | Same-seed 1-year `run_trial` gives identical `annals` before and after (a new test `refactor_keeps_a_year_identical` comparing death and siege notes); `one_year_timing` within ±3% | none |
| P1 W20 | Rerun `arena` and paste its table into the commit message | wave 1: siege kills, bleeding share |
| P2 A01 | `siege_replay` shows varied arrival | wave 1: arrival histogram |
| P3 B06a | `siege_path` pure tests; `one_year_timing` +≤10%; `SIEGE_PLANS` counter in the F3 overlay | wave 1: breaches, survival (watch closely) |
| P4 F05 | `where_the_flood_comes_from` (existing) still passes | wave 1: drowning |
| P10/P18 A02 | arena with morale on/off; a replay with a rout | wave 2/3: routs, yields |
| P11 B13a | `one_year_timing` +≤8%; a light-recompute counter | wave 2: timing, cavern deaths |
| P12/P23 W26 | `fluids` timing unchanged without pumps | wave 2/3: water deaths |
| P21 A05 | a replay with a ballista | wave 3: engine kills |
| P24 D01 | `cavern_floors` cache test | wave 3: cavern raids |
| P30 A06 | Trial outcome "surrendered" in report.rs tests | wave 4: surrenders, floods |
| P33 B12 | food per head in a 2-year trial ≥ before | wave 4: starvation |
| P31/P34 B03 | `expedition` tests | wave 4/5: expedition deaths <5% |
| P36 D02 | a `--style delver` trial reaches the tubes | wave 5: a Delver batch |

The coordinator's batch per wave: `--autoplay --runs 25 --years 3 --seed 5000 --out batches/m56-wN.json`,
then `--compare` against the previous wave. Any survival drop >10 points blocks the next wave
until it is tuned.

**Report additions needed** (report.rs, done in the package that introduces each field):
`SiegeNote.{ambush, breach, routs_ours, routs_theirs, yielded, cause}`, an arrival-day histogram,
the engine-kill share, a "surrendered" outcome, and new death causes (fell, froze, disease,
venom, demons).

---

## Totals

| Milestone | Items | Sessions |
|---|---|---|
| Foundations and refactors | — | 4 |
| M5 | A01 3, A02 5, W20 2, A03 3, A04 5, B06 6, A05 4, A06 3, B03 5 | 36 |
| M6 | F05 3, B13 4, W26 5, D01 3, D02 4, D03 3, D04 4, D05 5, D06 3, D07 4, D08 5, B11 5, D09 5, B12 4, D10 4, D11 3 | 64 |
| **Total** | 25 items | **≈104 sessions** (≈5-6 waves with 6-9 agents in parallel) |

---

## Cross-milestone notes

1. **One event queue (F-EV)** is the most valuable foundation. Death, kill, wound, siege, noise
   and breach are the inputs of A01-A03, A06, D01, D07, the chronicle, the O05 highlight reel
   and M3's grief and vengeance (C02). If M3 already built a memory or event type for C01/C02,
   extend that rather than adding a second.
2. **Captives (F-CAPT)** are shared by S03 (M1 captives step), B18 (M4), A01, A02, A06 and B03.
   Build it once, in whichever milestone gets there first, with the shape above.
3. **A civ-opinion store** (B02 diplomacy in M4: "stance" per civ, with grudges and reasons
   from B16) is read by A01 (who comes), A02 (execution grudges), A06 (surrender) and B03
   (retaliation, tribute). M5 should not invent its own: if B02 hasn't landed, add
   `Game.civ_stance: HashMap<u32, i32>` in F-CAPT's package.
4. **Sparse per-tile maps instead of `Tile` fields:** `Tile` is `Copy` and in a w×h×d Vec, so every
   byte there costs memory and cache. Use `HashMap<Pos, _>` for rare state (damage, stains, soil,
   webs, growths, snow depth). Only `wq` (W26) and `ice` (D11) belong on `Tile`, as
   `skip_serializing_if` zero.
5. **Path cost:** B06 (planner), D01 (fliers), D04 (large), W26 (magma movers) and B13 (darkness
   slowing) all touch pathing. Do the mover-table refactor first, keep the planner out of
   `path::search`, and add a `SIEGE_PLANS` counter beside `SEARCHES` in the F3 overlay. Rule:
   one plan per army, never one per creature.
6. **The overseer needs a "situation" layer.** M5 and M6 add about 12 new senses and troubles
   (Breach, Desperate, Occupied, Encircled, Rumbling, DeepMetal, Foretold, Sickness, Flooded,
   Cold, SameFood, Mutinous). `brain()` is already a long hand-written function. Refactor senses
   into a table `(Sense, fn(&Game, &Overseer) -> f64, links: &[(Ambition, f64)])` before M5,
   so each item adds a row.
7. **Duplicates across milestones to build once:** SIE-20/OVR-17, MIL-07/OVR-07, MIL-37/HLT-32,
   DIG-08/HLT-36/B13, DIG-10/AGR-03, DIG-18/AGR-22, ENG-26/F05 surge, DIG-20/W26 magma creatures,
   AGR-08/F05 melt, CRF-14/MND-46, MIL-13/C02 vow, MIL-19/B18 OUT-02.
8. **Balance gates:** B06 (siege AI) and A02 (dwarf routs) are the two changes most likely to send
   survival back towards the 0-of-25 baseline. Ship each behind a `Knobs` threshold (S10's
   tunable thresholds) so the batch can tune them without a rebuild.
