# Implementation plan: Milestone 7 "dread", Milestone 8 "a world of forts", and Later

Grounded in DwarfClone at October 2026 (src/game/mysteries.rs, justice.rs, necromancy.rs, combat.rs,
expeditions.rs, positions.rs, psyche.rs, overseer.rs, ambitions.rs, src/saves.rs, src/app.rs,
src/main.rs, src/overworld.rs). No repo files were edited.

## What the code gives us today (the starting points)

- **Clock.** `DAY = 1200`, `MONTH = DAY*28` (game.rs:100). A 28-day month is a lunar month: the full
  moon is day 14 of every month, so no new state is needed for a moon. There is **no day/night cycle**:
  dwarves sleep by fatigue (`Task::Sleep`, `d.asleep`, game.rs:2740). Night has to be defined
  (proposed below as the last sixth of each day plus "most of the fort is asleep").
- **Season hooks.** `Game::step` (game.rs:1014) calls each system by hand: every 100 ticks
  `expedition_tick / duels_tick / mysteries_tick`; daily blocks at `DAY/2+7` and `0`; seasonal block at
  `SEASON` (`maybe_siege, maybe_megabeast, maybe_cave_horror, maybe_thief, maybe_visitor, maybe_undead,
  maybe_stranger, ...`). Every M7/M8 item wants a line there; that is the main merge hot spot.
- **Mysteries** (mysteries.rs) are staged stories: `Mystery { kind, stage, dwarf, thing, next }`,
  a `match` per kind, `mysteries_done` so each happens once. The hooded stranger already has a goblin
  spy ending that sets `siege_warned` (the seed of B04/N02). The pattern is good and should be reused
  for vampire hunts, cursed-site investigations and prophecies.
- **Justice** (justice.rs) is a crime list with a known culprit (`Crime { kind, culprit, victim, what,
  tick, punished }`), a sheriff `Task::Punish`, jail via `jailed_until`. There is no idea of an
  *unknown* culprit, suspicion, witness or accusation; C07 (Milestone 3) plans witnesses and wrong
  accusations. M7 needs exactly that, generalised (the "Case" store below).
- **Hostile creatures** (combat.rs) are `Creature`s with flags (`siege`, `sneaking`, `loot`, `general`,
  `label`, `desc` used as a tag such as `"risen:<name>"`). `creature_think` dispatches by species
  (Merchant, Skulker, Visitor) then generic aggro/siege AI that only ever targets **dwarves**. Creatures
  never fight creatures: this matters for mercenaries (P04) and werebeasts.
- **Skulkers** (`maybe_thief`, `skulker_think`, combat.rs:680) take the best free item and run; seen
  within 5 tiles they bolt. N02 grows out of this.
- **Necromancer** (necromancy.rs) is the template for a named off-map villain with `known/alive`, a
  seasonal roll and a spotlight. **Cave horrors** (`make_cave_horror`) are generated names and looks;
  `Creature::legendary()` covers CaveHorror/Undead/figures.
- **Expeditions** (expeditions.rs) *remove* the squad's `Dwarf`s from `game.dwarves` into
  `Expedition.members` and put them back on return. That is the trick for werebeast transformations,
  outposts, and **survivors of a fallen fort** (an expedition away when the fort falls survives it).
  Only one expedition at a time (`expedition: Option<Expedition>`).
- **Nobles** (positions.rs, psyche.rs): appointed posts (`Position`), titles Baron/Count/Duke granted by
  wealth+population (`TITLES`, `titles_daily`), furniture `Demand`s, mandates. No succession, no
  monarch (`Noble` enum in dwarf.rs:371 has Leader, Mayor, Baron, Count, Duke).
- **The world.** `Overworld` (overworld.rs:385) lives **inside every fortress save** (`Game.overworld`);
  there is no world file. History runs in the private `Overworld::simulate_year` driven only by
  `WorldGen::step`. The player's fort is **not** a `Site` (nothing outside overworld.rs pushes sites);
  `embarkable` only refuses cells with a site. `legend()` writes `overworld.events` with
  `start_year + tick/YEAR`, but `overworld.year` never moves during play.
- **Saves** (saves.rs) are one JSON per fortress name + autosave, plus `.meta.json`. `after_load`
  (ruins.rs:32) is the upgrade hook for old saves.
- **Game over** (app.rs:459) sets `game.fallen`, switches to `Mode::GameOver`; keys c/l/r/t/w. Nothing
  is written back to the world.
- **Batch** (`main.rs:autoplay_batch` → `overseer.rs:run_trial(seed, style, years)`) always builds a
  fresh world with `Game::new(seed)`; `Trial` is the serde record that `report.rs` tallies.
- **Overseer brain** (ambitions.rs: `Sense`, `Brain`, `brain()`), traits (personality.rs: Cautious,
  Bold, Vain, Pious, Greedy, Sentimental, Stubborn, Cheerful), troubles are *shortages* only
  (`troubles::Lack`). `oversee()` calls `ov_rooms / ov_orders / ov_posts / ov_army / ov_daily`.

---

## Interface contracts (shared foundations, build first)

These are used by several items in both milestones. Each gets its own new module so that the
foundation branch touches the hot spots once and dependents work against a fixed API.

### FC1 `src/game/clock.rs` (night and moon) — used by N01, N02, N03, B04

```rust
impl Game {
    /// Day of the lunar month, 0..28; the moon is full on day 14.
    pub fn moon_day(&self) -> u64 { (self.tick % MONTH) / DAY }
    pub fn full_moon(&self) -> bool { self.moon_day() == 14 }
    /// The last sixth of each day (dusk to midnight), the hours sneaks and night things keep.
    pub fn is_night(&self) -> bool { self.tick % DAY >= DAY * 5 / 6 }
    /// Night, or more than half the adults asleep.
    pub fn fort_sleeps(&self) -> bool;
    /// A dwarf outdoors with nobody within `r` tiles.
    pub fn alone_outside(&self, d: &Dwarf, r: i32) -> bool;
}
```
No saved state. UI: a moon glyph next to the date in the status bar (ui.rs, one span).

### FC2 `src/game/cases.rs` (suspicion, clues, accusations) — used by N01, B04, N02, N03, C07

The common "who did it" store. If C07 (M3) has landed with witnesses, this *is* its store; if not, the
M7 foundation builds it and C07 later reuses it.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseKind { Draining, Murder, Theft, Fouling, Sabotage, Spying, Desecration, Missing }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clue { pub suspect: u32, pub weight: i32, pub what: String, pub from: Option<u32>, pub tick: u64 }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub id: u32,
    pub kind: CaseKind,
    pub victim: Option<u32>,
    pub at: Option<Pos>,
    pub opened: u64,
    /// The truth, hidden from the player and overseer (None = an accident).
    pub culprit: Option<u32>,
    pub clues: Vec<Clue>,
    /// Who was accused (rightly or not), and when.
    pub accused: Option<(u32, u64)>,
    pub closed: bool,
}

impl Game {
    pub fn open_case(&mut self, kind: CaseKind, victim: Option<u32>, at: Option<Pos>, culprit: Option<u32>) -> u32;
    pub fn add_clue(&mut self, case: u32, suspect: u32, weight: i32, what: String, from: Option<u32>);
    /// Suspects by summed clue weight, most suspected first.
    pub fn suspects(&self, case: u32) -> Vec<(u32, i32)>;
    /// Accuse: files a Crime against them (justice.rs) and records whether it was right.
    pub fn accuse(&mut self, case: u32, suspect: u32) -> bool;
    pub(super) fn cases_daily(&mut self); // witnesses near a case add clues; old cases go cold
}
```
Storage: `Game.dread.cases: Vec<Case>` (see FC5). `accuse` calls `report_crime` so the existing sheriff
punishes; a new `CrimeKind::Murder | Spying | Sabotage | Desecration` is added to justice.rs (serde
enum variants are additive).

### FC3 `src/game/curses.rs` (curse framework, MYT-37) — used by N01 (were/vampire), N03, B04

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CurseKind { Were { beast: String }, Vampire, BadLuck, Wasting, Haunted, Oathbreaker, Pact { price: PactPrice } }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trigger { Always, FullMoon, Night, NearItem(u32), Season(u8) }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Lift { Never, DestroyItem(u32), CarryAway(u32), PrayAt(ZoneKind), Bathe, Kill(u32), Rebuild(BuildingKind) }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Curse { pub kind: CurseKind, pub source: String, pub trigger: Trigger, pub lift: Lift, pub since: u64, pub known: bool }

impl Game {
    pub fn lay_curse(&mut self, dwarf: u32, c: Curse);
    pub fn lift_curse(&mut self, dwarf: u32, why: &str);
    pub(super) fn curses_tick(&mut self);  // every 100 ticks: triggers fire, lifts checked
}
```
Storage: **one** field on `Dwarf`: `#[serde(default)] pub curse: Option<Box<Curse>>` (boxed to keep
`Dwarf` small). A dwarf carries at most one curse; a second one replaces a weaker one.

### FC4 `src/game/secrets.rs` (hidden identities and plots) — used by N01, B04, N02

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Secret { Vampire { thirst: u32, fed: u64 }, Spy { civ: u32 }, Agent { plot: u32 }, Traitor { grudge: String }, Cultist { cult: u32 } }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlotGoal { StealArtifact(u32), Assassinate(u32), OpenGate, FoulWater(u32), Intel }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plot { pub id: u32, pub master: String, pub civ: Option<u32>, pub goal: PlotGoal,
                  pub agents: Vec<u32>, pub stage: u8, pub next: u64, pub exposed: Vec<u32>, pub done: bool }
```
Storage: `Dwarf.secret: Option<Box<Secret>>` (serde default) and `Game.dread.plots`.
`Secret` is hidden on the unit sheet until `exposed`; the UI shows only what the fort knows.

### FC5 `src/game/dread.rs` (the M7 container and its three hooks)

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Dread {
    #[serde(default)] pub cases: Vec<Case>,
    #[serde(default)] pub plots: Vec<Plot>,
    #[serde(default)] pub omens: Vec<Omen>,
    #[serde(default)] pub pending: Vec<(u64, Threat)>,   // threats decided, not yet arrived
    #[serde(default)] pub banes: Vec<(String, Bane)>,    // what the fort has learned
    #[serde(default)] pub hidden: Vec<Dwarf>,            // dwarves off the list (in beast form)
    #[serde(default)] pub fouled: Vec<(u32, u64, Option<u32>)>, // well, until, by
    #[serde(default)] pub exiles: Vec<Exile>,
    #[serde(default)] pub taken: Vec<Taken>,
    #[serde(default)] pub prophecies: Vec<Prophecy>,
    #[serde(default)] pub site_curse: Option<SiteCurse>,
    #[serde(default)] pub tally: DreadTally,              // counters for the batch report
}
impl Game {
    pub(super) fn dread_tick(&mut self);            // every 100 ticks (offset 40)
    pub(super) fn dread_daily(&mut self);           // once a day
    pub(super) fn dread_season(&mut self, season: u64);
}
```
`Game` gets **one** field: `#[serde(default)] pub dread: dread::Dread`. `step` gets **three** lines.
Every M7 item registers its work inside these three functions (one line per subsystem), never in
`step` directly. `DreadTally` counts vampires/spies caught, ticks to catch, wells fouled/traced,
curses laid/lifted, night-creature kills, wrong accusations; `run_trial` copies it into `Trial.dread`.

### FC6 `src/world_record.rs` + `saves.rs` world file (P01, used by all of M8)

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Fate { Fell { year: i32, killer: Option<String> }, Retired { year: i32 }, Abandoned { year: i32 } }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FortRecord {
    pub name: String, pub site: (i32, i32), pub site_id: u32, pub civ: Option<u32>,
    pub founded: i32, pub fate: Fate, pub save_file: String, pub peak_pop: usize,
    pub artifacts: Vec<String>, pub journal: Vec<String>,     // chronicle titles, overseer notes
    pub occupant: Option<Occupant>, pub reclaimed_by: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Occupant { Army { civ: u32, n: u32 }, Beast { figure: u32 }, Horror { label: String }, Dead, Empty }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldRecord {
    pub seed: u64, pub overworld: Overworld, pub forts: Vec<FortRecord>,
    /// Dwarves who outlived a fort, waiting to turn up as migrants.
    pub wanderers: Vec<(Dwarf, String)>,      // (dwarf, "survived the fall of X")
}
// saves.rs
pub fn write_world(dir: &Path, w: &WorldRecord) -> Result<String, String>;  // "<World>.world.json"
pub fn read_world(dir: &Path, name: &str) -> Result<WorldRecord, String>;
pub fn list_worlds(dir: &Path) -> Vec<(String, usize)>;
// overworld.rs
impl Overworld {
    pub fn advance(&mut self, years: i32, seed: u64);          // public wrapper on simulate_year
    pub fn found_fort(&mut self, name: &str, civ: Option<u32>, x: i32, y: i32, year: i32) -> u32; // site id
    pub fn fort_fell(&mut self, site: u32, year: i32, occupant: &Occupant, text: String);
}
// game.rs
impl Game {
    /// Close this fortress into the world: its site, its legend, its survivors.
    pub fn write_back(&self, rec: &mut WorldRecord, fate: Fate);
    /// A new expedition into an old fort's map, years later.
    pub fn reclaim(old: Game, ow: Overworld, seed: u64, years: i32) -> Game;
}
```
`Game` gets `#[serde(default)] pub site_id: Option<u32>` and `#[serde(default)] pub world_file:
Option<String>`. If B17 (M4, "the world turns each year") has landed with its own yearly pass,
`advance` simply calls it N times.

### FC7 `src/game/realm.rs` (the M8 container: crown, missions, money, routes)

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Realm {
    #[serde(default)] pub favour: i32,                 // with the crown (-100..100)
    #[serde(default)] pub monarch: Option<u32>,        // dwarf id once they move in
    #[serde(default)] pub claims: Vec<Claim>,          // succession disputes
    #[serde(default)] pub missions: Vec<Mission>,      // scouts, aid, messengers, own caravans
    #[serde(default)] pub outposts: Vec<Outpost>,
    #[serde(default)] pub explored: Vec<u64>,          // bitset over overworld cells; empty = all known
    #[serde(default)] pub coffers: i64,                // abstract money
    #[serde(default)] pub contracts: Vec<Contract>,    // mercenaries, patrons, loans, rackets
    #[serde(default)] pub routes: Vec<Route>,
    #[serde(default)] pub tally: RealmTally,
}
impl Game { pub(super) fn realm_daily(&mut self); pub(super) fn realm_season(&mut self, season: u64); }
```
`Mission` generalises `Expedition` (see hot-spot refactor H3): `Mission { kind: MissionKind, members:
Vec<Dwarf>, cargo: Vec<Item>, dest: Destination, back: u64, route: Vec<(i32,i32)> }` with
`MissionKind::{Expedition(Goal), Scout, Messenger(String), Aid(u32), Caravan(u32), Outpost(u32)}`.

### FC8 Bandit camps (shared by N02, P03, P04, P05)

`SiteKind::BanditCamp` added to overworld.rs (name "bandit camp", glyph '¤'), with
`Overworld::bandit_camps_near(x, y, r) -> Vec<u32>` and `Overworld::spawn_bandits(rng, year)` (called
from `advance`, and from B17's refugees step WLD-21 if present). A bandit raid on the fort reuses
`lay_siege_of` with a synthetic `Neighbor { civ: u32::MAX, race: Race::Human, dist }`, so
`at_war_with` must treat `civ == u32::MAX` as "always hostile" (one guard line).

---

## Hot-spot discipline (refactors to land before the parallel waves)

Each is a small, behaviour-neutral commit with the existing tests green. Land them as **wave 0**, by
one agent, in this order.

- **H1 System hooks in `Game::step`.** Move the seasonal list in game.rs:1112 into
  `fn season_systems(&mut self, season: u64)` and the daily list into `fn daily_systems(&mut self)`, one
  call per line, with an empty `// M7` and `// M8` line each where `self.dread_season(season)` /
  `self.realm_season(season)` go. Branches then add exactly one line in a function that nobody else
  rewrites.
- **H2 Container fields.** Add `dread: Dread` and `realm: Realm` (empty structs with `Default`) to
  `Game` and their three hooks as no-ops, plus `Dwarf.curse` and `Dwarf.secret` (`Option<Box<..>>`,
  serde default). After this, **no M7/M8 branch adds a field to `Game` or `Dwarf`**; they add fields
  to `Dread`, `Realm`, `Curse`, `Secret` in their own modules.
- **H3 Missions.** Turn `expedition: Option<Expedition>` into `missions` on `Realm` with a serde shim:
  keep the old field as `#[serde(default)] expedition: Option<Expedition>` and move it into
  `realm.missions` in `after_load`. `expedition_tick` becomes `missions_tick` that dispatches on
  `MissionKind`. Existing `send_expedition`/`destinations` keep their signatures. (Needed by P01
  survivors, P03, P05; B18 in M4 may already have done this; if so skip.)
- **H4 Creature think dispatch.** In combat.rs:`creature_think` replace the `if species == ...`
  ladder with `match c.species { Merchant => .., Skulker => .., Visitor => .., Werebeast | Bogeyman |
  Kobold => self.dread_creature_think(c), _ => generic }` so night creatures live in dread modules.
- **H5 Strike hook.** In combat.rs:`strike` add one call `let mult = self.strike_mods(att, def);`
  (a no-op returning 1.0, in a new `src/game/strike_mods.rs`). Banes (N01), artifact powers (N03) and
  pacts add arms to `strike_mods` instead of editing `strike`.
- **H6 Overseer hook.** In overseer.rs:`ov_daily` add `self.ov_dread(o)` and `self.ov_realm(o)`
  (no-ops in new `src/game/ov_dread.rs`, `src/game/ov_realm.rs`), and in ambitions.rs make the sense
  list extensible: `Sense::Dread` and `Sense::Realm` variants added now (values 0.0) with their link
  rows, so branches only change the value functions in their own modules
  (`fn dread_sense(&self) -> f64`, `fn realm_sense(&self) -> f64`).
- **H7 UI split.** ui.rs is 5,131 lines. Before the waves move the nobles screen and legends screen
  into `src/ui/nobles.rs` and `src/ui/legends.rs` (pure moves), and add an empty
  `src/ui/intrigue.rs` and `src/ui/realm.rs` with one tab registration each. New screens then never
  conflict in ui.rs.
- **H8 Trial record.** Add `#[serde(default)] pub dread: DreadTally` and `#[serde(default)] pub realm:
  RealmTally` to `Trial`, and in report.rs a `fn dread_section(&[Trial]) -> String` and
  `fn realm_section` that print nothing until counters are non-zero. Branches fill counters only.
- **H9 run_trial split (for P01).** Split `run_trial(seed, style, years)` into
  `run_fort(g: Game, style, years) -> (Trial, Game)` plus the old wrapper, so the chain batch can keep
  the fallen `Game` and the world.

---

## Milestone 7: dread (v0.9)

**Order and why.** The WISHLIST order is N01, B04, N02, N03, but N01 (werebeasts, vampires) and B04
(spies) both stand on the curse framework and the case store, which WISHLIST files under N03
(MYT-37) and C07. So: **foundations first (FC1-FC5 + H1-H8)**, then N01 and B04 in parallel, N02 after
B04's plots exist (assassins, exiles and spies are shared), N03 last but its first step (curse
framework) is in the foundation.

- Critical path: wave 0 refactors → FC2 cases + FC3 curses + FC4 secrets → N01 vampire (the Done-when
  needs cases + secrets + clock) → N02 fouled well (cases) → release batch.
- Parallel: N01 werebeasts/banes/bogeymen and N03 relics/places of power touch disjoint modules.
- Dependencies on earlier milestones: **C07** (witnesses, banishment NOB-03; exile return needs
  banishment), **C10** (temples, priests; N03's Done-when needs a temple with a statue), **D11** (sky
  omens and prophecies WLD-08 — same Prophecy type), **B21** (baby snatchers, kobolds — N02 must not
  duplicate), **D05** (sickness, for poisoned water), **A06** (martial law). Where a dependency has
  not landed, the step names the minimal stand-in.

**Release check.**
- Every Done-when test passes: a migrant vampire drains sleepers until the fort accuses it
  (`dread::tests::a_vampire_among_the_migrants_is_found_out`); a caught spy names the plot
  (`secrets::tests::a_spy_interrogated_names_the_plot`); a fouled well is traced to the saboteur
  (`shadows::tests::a_fouled_well_is_traced_back`); a dwarf toppling a temple statue in a tantrum is
  cursed (`curses::tests::toppling_a_temple_statue_brings_a_curse`).
- Batch `--autoplay --runs 25 --years 3 --seed 5000 --out batches/m7.json`, compared with the M6
  release batch: at least 40% of forts meet an inside threat (vampire, spy, saboteur, curse); of
  vampires that arrive, 60% are caught before their third kill; wrong accusations 10-30% of
  accusations (doubt is the point); night creatures and plots cause no more than 12% of all deaths;
  year-3 survival within 10 points of M6. Watch one `--story` fort with a vampire to read the arc.

### N01 Things in the night  (effort: 4 sessions; risk: med)

- **Design:**
  - *Omens (MYT-05):* threats are decided days before they arrive. `dread.pending: Vec<(u64, Threat)>`
    with `enum Threat { Siege, Megabeast(u32), Werebeast, Horror, Undead }`. `maybe_megabeast`,
    `maybe_undead` and the new werebeast roll push into `pending` with an ETA 3-6 days out instead of
    spawning; `dread_tick` spawns when due. `maybe_siege` already has `incoming` (one day); the omen is
    rolled when `incoming` is set and, for armies of 8+, the siege ETA grows to 2-3 days. An omen is
    `Omen { kind: OmenKind, threat: Threat, tick, read: bool }`, `OmenKind::{BlackStag, BloodMoon,
    SilentBirds, StormFromNowhere, MilkSours, DogsHowl}` (D11 sky events, if present, are added as
    kinds). Dwarves who see one (outside, or any for in-fort omens) get a thought scaled by
    `personality.anxiety`-like facet (use the existing stress facet, or pious need strength).
  - *Banes (MYT-06):* `enum Bane { Iron, Steel, Fire, Obsidian, Daylight, RunningWater }`; each
    supernatural species/curse has one (`fn bane_of(&Creature) -> Option<Bane>`; werebeast = iron
    or steel per world seed, vampire = fire/daylight, undead = obsidian, bogeyman = light/company).
    In `strike_mods` (H5): a bane weapon does x2.5 damage; non-bane weapons x0.6 against werebeasts.
    The first bane hit records `dread.banes.push((kind, bane))`, a legend and a spotlight "Iron bites
    the beast!". Known banes show in the legends' "Lore" tab.
  - *Werebeast raid and moon curse (MYT-03):* `Species::Werebeast` (creature.rs: fast, strong, claws,
    `aggro` high). On a full-moon night (`full_moon() && is_night()`) with a world werebeast figure
    nearby (or 15% per year anyway in savage/evil lands), one arrives from the edge, mauls (each bite
    on a dwarf: 40% chance `lay_curse(Curse { kind: Were{beast}, trigger: FullMoon, lift: Never })`)
    and leaves at dawn (`leave_tick = next morning`). A cursed dwarf on the next full moon: stashed in
    `dread.hidden` (the expedition trick) and replaced by a `Werebeast` creature with
    `desc = "were:<id>"` at their spot; at dawn the creature despawns and the dwarf comes back naked
    (clothes dropped), with "woke ashamed among the blood" and every witness's regard falling. If the
    beast-form is slain the dwarf dies (`DeathNote.cause = "slain in beast form"`).
  - *Secret vampire citizen (MYT-04, also NOB-24):* in `migrants()` (game.rs:1279) a 4% roll per wave
    after year 1 (higher in evil worlds) gives one migrant `Secret::Vampire` and a curse
    `Vampire`. Vampires: never age in `new_year` (family.rs:208), never eat or drink (needs.rs gate),
    sleep in the day. Feeding: `Task::Feed { victim }` chosen in `dread_tick` when `fort_sleeps()` and
    a victim is asleep within reach: victim gets `body.wound(torso, 2, bleed)` and "woke pale and weak";
    `thirst` rises each day unfed; the thirstier, the more blood taken, and past a threshold the victim
    dies (opens `Case { kind: Draining }`). Sloppy: witnesses within 6 tiles add clues; the vampire
    moves the body to an unvisited corner (hide). Detection clues, all through FC2: seen at a
    sleeper's bed (+30), never seen eating or drinking (+5 a week), unchanged age at new year (+20),
    "felt a chill" near them (+10, sensing evil). The sheriff (or overseer, H6) accuses when the top
    suspect has 60+ and twice the next; if accused rightly, the vampire flees and fights (bane: fire).
  - *Bogeymen (MYT-07):* only in evil cells or a cursed site (FC: `self.evil ||
    dread.site_curse.is_some()`): a dwarf `alone_outside(d, 12)` at night may be stalked: three
    `Species::Bogeyman` (cackling shapes) spawn adjacent, target only that dwarf, vanish at dawn or if
    another dwarf comes within 5. Overseer reaction: curfew (no outdoor jobs at night) in that fort.
  - *Sensing evil (MYT-26):* in `dread_daily`, for each hidden evil (vampire, were in dwarf form,
    ghost, unseen cave horror within 15 tiles) each dwarf near it rolls by a sensitivity (pious +
    focus); a hit is gossip "felt a chill near X" and a clue (+10 on X) for any open case.
  - *Charms and wards (MYT-25):* `ItemKind::Charm` (Craftsdwarf recipe, bone or gem), worn like
    clothing; `Building.warded: bool` (serde default) set by an engraver on a door. Whether they work
    is a world fact: `Overworld.wards_work: Option<bool>` (serde default None → rolled in
    `after_load`, 50%). If they work, bogeymen and werebeasts will not path through warded doors
    (`Mover` check in their think), and charm-wearers are skipped by bogeymen; if not, they only
    lower stress (placebo) and the legends never say which.
- **Steps:**
  1. Omens: `Threat`, `Omen`, `pending` scheduler; `maybe_megabeast/maybe_undead` defer · dread.rs,
     combat.rs:maybe_megabeast (one call change), necromancy.rs:maybe_undead · test
     `an_omen_comes_days_before_the_beast` (force a megabeast roll, assert omen logged ≥2 days before
     the creature exists).
  2. Banes: `Bane`, `strike_mods` arm, learning · strike_mods.rs, dread.rs · test
     `iron_bites_the_werebeast_and_the_fort_remembers` (arena duel iron axe vs copper axe, compare
     damage; `dread.banes` non-empty after).
  3. Werebeast raid and moon curse · creature.rs (species row), new `src/game/night.rs` (raid,
     transform, dawn) · test `a_bitten_dwarf_turns_at_the_full_moon_and_wakes_ashamed`.
  4. Secret vampire: migrant roll, no aging/eating, feeding, hiding the body · night.rs, game.rs:migrants
     (one call `self.maybe_vampire(&mut came)`), family.rs:new_year (one `if` for vampires) · test
     `a_vampire_feeds_on_sleepers`.
  5. The hunt: clues and accusation into FC2, flee on accusation · night.rs, cases.rs · Done-when test
     `a_vampire_among_the_migrants_is_found_out` (seed a vampire, run 60 days with a sheriff, assert
     it was accused and at least one sleeper was drained first).
  6. Bogeymen · night.rs, creature.rs · test `a_dwarf_alone_outside_at_night_in_evil_land_is_stalked`.
  7. Sensing evil · night.rs · test `dwarves_feel_a_chill_near_the_vampire`.
  8. Charms and wards · items.rs (Charm kind), buildings.rs (`warded`), night.rs · test
     `wards_keep_the_bogeymen_out_when_the_world_says_they_work`.
- **Overseer & screen:** `Sense::Dread` (H6) fires from unread omens, open Draining/Murder cases and
  known night creatures; links: Fortify +0.8, Muster +0.6, Revel -0.4. `ov_dread`: appoints a sheriff
  as soon as a case opens (Cautious immediately, Bold only after a death), accuses the top suspect
  (Cautious waits for 80, Bold accuses at 50: more wrong accusations), equips the squad with the bane
  weapon material when known, sets the curfew in evil lands, has charms made if Pious. Screen: omen
  spotlights ("A black stag watched the gate at dusk"), a full moon glyph on the date, a "Lore" tab in
  legends with banes, the vampire's unmasking as a spotlight and chronicle line ("the Year of the
  Pale Guest"), the transformed dwarf shown as a red `W`.
- **Measure:** batch counters `vampires`, `vampires_caught`, `days_to_catch`, `drained`, `were_bitten`,
  `night_deaths`. Targets: vampire caught in 60%, median under a season; night deaths ≤ 6% of deaths.
- **Risks / open questions:** the dwarf-as-creature swap must keep relations, items and burrows; it
  must also be excluded from `alive_count()` checks so a fort of one were-dwarf at full moon is not
  "fallen" (count `dread.hidden` as alive). Vampire feeding needs a cheap victim search (only on
  `fort_sleeps`, every 100 ticks). Decision for the user: are vampires ever *kept* (DF players lock
  them in as immortal administrators)? Proposed: a Greedy overseer may make a caught vampire its
  bookkeeper instead of jailing it.
- **Parallel work:**
  - **Writes:** new `src/game/night.rs`, `src/game/clock.rs`; `dread.rs` (Omen/pending parts);
    `strike_mods.rs` (bane arm); creature.rs `Species` + `info()` rows (Werebeast, Bogeyman);
    items.rs `ItemKind::Charm`; buildings.rs `Building.warded`; overworld.rs `wards_work`.
  - **Reads / depends on:** FC1-FC5, H1-H6. `open_case/add_clue/accuse`, `lay_curse`, `Secret::Vampire`.
  - **Hot spots:** game.rs `migrants` (one call), family.rs `new_year` (one `if`), needs.rs eat/drink
    gate (one `if d.is_vampire()`), combat.rs `maybe_megabeast` (one line to defer), necromancy.rs
    `maybe_undead` (one line), creature.rs enum (append only).
  - **Agent size:** two agents: N01a omens+banes+werebeasts+bogeymen (steps 1-3, 6), N01b vampire
    (steps 4, 5, 7) and wards (8). They share `night.rs`: give N01b `src/game/vampire.rs` instead.

### B04 Enemies within  (effort: 5 sessions; risk: high)

- **Design:**
  - *Questers in the tavern (NOB-29):* visitors.rs gets `QUESTER: u8 = 6`. They come when the fort holds
    an artifact (`quality >= ARTIFACT`), sit in the tavern, and each chat with a dwarf of sociability
    > 20 leaks the artifact's room (`Plot { goal: StealArtifact }` advances a stage). Then either a
    petition (overseer accepts by trait: Greedy never, Sentimental sometimes; accepted = artifact leaves
    with them, the civ's favour rises) or a theft at night (skulker behaviour on that one item).
  - *Spies and double agents (NOB-25):* `migrants()` and visitors may carry `Secret::Spy { civ }` (rate
    scaled by enemies within 30 and by wealth). A spy's plot is `Intel`: each season it adds intel
    (`stage += 1`); at stage 2 `siege_warned = true` *and* the next siege of that civ arrives on the
    side opposite the gate or with a known way in (the gate is held open once). Interrogated and
    turned, a spy becomes a double agent: the next siege of that civ is delayed a season and smaller.
  - *Interrogation (NOB-21):* a new sheriff task `Task::Interrogate { suspect }` (in justice.rs's
    neighbour, new `src/game/interrogation.rs`). Method by the sheriff's personality: temper ≥ 20
    intimidate, sociability ≥ 20 persuade, otherwise flatter. Odds from the suspect's stubbornness and
    regard for the sheriff; outcomes: confess and name the next link (`add_clue(+60)` on another
    agent, plot `exposed.push`), say nothing, or name an innocent (a wrong lead with +30 weight). Caged
    goblins (traps.rs cages) can be interrogated for the next siege's size and leader.
  - *Traitorous advisor (NOB-28, OVR-23):* a post-holder (`positions`) with `Secret::Traitor` (from a
    grudge: unmet demand, punished kin, or bribed by a plot). Broker: skims 10-20% of each trade's
    value into nothing; Manager: quietly drops one defence work order a season (the overseer's notes
    say "dropped the mechanisms order on X's advice"); Bookkeeper: misreports one stock (the
    overseer's `stock_of` used in `ov_troubles` goes through `ov_seen_stock()` which applies the lie).
    Found by audit (a second good bookkeeper, interrogation, or N03's thought-reading helm).
  - *Villain networks (NOB-26):* a world villain = `Plot.master` naming an overworld figure (a goblin
    ruler, the necromancer, a bandit chief). The master recruits through agents: bribe (Greedy-like
    dwarves: low regard for the fort's leader, high stress), blackmail (a dwarf with a closed Crime),
    flattery (vain = high self-regard: use `personality` facet). A turned dwarf becomes
    `Secret::Agent { plot }` and steals the artifact to a cache at the map edge for a skulker pickup.
  - *Assassination plots (NOB-27):* `PlotGoal::Assassinate(noble or hero)`: poison (food item
    `Item.tainted: Option<u32>` serde default — shared with N02), ambush when the target is alone
    (agent attacks: reuse tantrum-punch code path with lethal force), or an "accident" (pushed into
    water/off a ledge near a channel). Each opens a Case with the agent as hidden culprit.
  - *Counterintelligence screen (NOB-61):* `src/ui/intrigue.rs`, a tab on the nobles screen: open
    cases with suspects and clue lines; known agents drawn as a tree to their master ("?" until
    exposed); actions `a` accuse, `i` interrogate (sheriff), `x` exile (needs C07 banishment; stand-in:
    "sent away" removes the dwarf and records an `Exile`).
  - *Exile returns (NOB-33, also N02 SIE-28):* `dread.exiles: Vec<Exile { dwarf: Dwarf, grudge: String,
    left: u64, joined: Option<u32> }>`. One to three years later an exile joins a hostile neighbour's
    siege as its leader (`lay_siege_of(.., leader: Some(name))`) and "knows the way": the army spawns
    at the nearest breach or holds the gate's door open once. Recognised: old friends get "saw X among
    the attackers", kin may refuse to fight.
- **Steps:**
  1. Questers and artifact talk · visitors.rs (one const + one arm), new `src/game/plots.rs` · test
     `a_quester_learns_where_the_artifact_is_kept_and_asks_for_it`.
  2. Spies among migrants and visitors, intel feeding sieges · plots.rs, game.rs:migrants (shared one
     call with N01: `self.migrant_secrets(&mut came)`), combat.rs:maybe_siege (one line reading
     `dread.intel_for(civ)`) · test `a_spy_brings_the_siege_by_the_back_way`.
  3. Interrogation (dwarves and caged creatures) · new `interrogation.rs`, justice.rs (task dispatch
     one arm in `find_punishment`), game.rs Task enum (append variant) · test
     `interrogation_can_name_the_next_link_or_an_innocent`.
  4. Double agents · plots.rs · test `a_turned_spy_delays_their_army`.
  5. Traitorous advisor · plots.rs, trade.rs (one call in the trade settlement), orders.rs (one hook),
     troubles.rs `ov_seen_stock` · test `a_traitor_broker_skims_until_found_out`.
  6. Villain networks and recruitment · plots.rs · test `an_agent_is_recruited_and_steals_the_artifact`.
  7. Assassination plots · plots.rs, items.rs `tainted` · test `a_poisoned_meal_kills_the_baron_and_opens_a_case`.
  8. Counterintelligence screen · ui/intrigue.rs, app.rs (keys on the nobles screen tab) · Done-when
     test `a_spy_is_caught_and_interrogated_and_names_the_plot` (logic) + a ui render test (string
     contains the master's name once exposed).
  9. Exile returns · plots.rs, combat.rs `lay_siege_of` (leader param exists) · test
     `an_exile_leads_the_next_siege_and_knows_the_way`.
- **Overseer & screen:** `ov_dread` reads `dread.plots`/cases: Cautious overseers interrogate every
  stranger, Bold ones ignore spies until a siege, Greedy accept bribes for the artifact (rarely),
  Sentimental refuse to exile kin. A traitor manager distorts the overseer's own decisions, visible in
  its notes ("on Urist's advice"). Screen: spotlight on each unmasking, gossip lines, the intrigue tab,
  chronicle titles ("the Year of the Turned Broker").
- **Measure:** batch counters `spies`, `spies_caught`, `plots`, `plots_exposed`, `artifacts_stolen`,
  `assassinations`, `wrong_accusations`. Targets: a plot in 30% of 3-year forts; half of plots exposed;
  sieges guided by a spy kill at most 1.5 more dwarves than others (do not make it a death sentence).
- **Risks / open questions:** the biggest item in M7: it touches justice, trade, orders, troubles and
  sieges. Wrong accusations punishing innocents can spiral stress; cap one accusation per case and give
  the falsely jailed a "was wronged" thought that fades. The traitorous advisor that lies to the
  overseer is subtle and could hide real shortages and starve forts; keep the lie to non-food stocks.
  User decision: should the player see secrets the fort does not know (a cheat toggle in the
  intrigue tab)? Proposed: hidden, with a debug env var `TRIAL_SECRETS` for the batch story.
- **Parallel work:**
  - **Writes:** new `src/game/plots.rs`, `src/game/interrogation.rs`, `src/ui/intrigue.rs`;
    visitors.rs (QUESTER arm), justice.rs (`CrimeKind` variants + interrogation dispatch),
    trade.rs (skim hook), orders.rs (drop-order hook), troubles.rs (`ov_seen_stock`), items.rs
    (`tainted`), app.rs (intrigue keys).
  - **Reads / depends on:** FC2 (cases), FC4 (secrets/plots), H1-H7. C07 banishment for exile (else
    the stand-in). N03's thought-reading helm is optional.
  - **Hot spots:** game.rs `Task` enum (append `Interrogate`, `Feed` goes to N01: coordinate append
    order in wave 0 by adding both variants in H2), `migrants` (shared single call
    `migrant_secrets`, created in H2), combat.rs `maybe_siege` (one line), app.rs (one key arm on the
    nobles screen).
  - **Agent size:** too big for one; split B04a plots+spies+questers+double agents (1,2,4,6), B04b
    interrogation+traitor+assassination (3,5,7), B04c screen+exile (8,9) after a and b.

### N02 Shadows at the door  (effort: 4 sessions; risk: med)

- **Design:**
  - *Sneaks before armies (SIE-07):* combat.rs:maybe_siege: until `fort_worth()` (wealth + 500×pop)
    passes a threshold or the fort is a year old, a goblin civ sends a sneak party instead (two thieves
    or a kidnapper) — the S03 raiding party of 2-3 stays for later years. A young outpost thereby
    gets a gentler first year (the WISHLIST baseline says most forts fall in year 1).
  - *Thieves who plan (SIE-44):* `skulker_think` (combat.rs:697) picks the target with the fewest dwarves
    within 10 tiles, waits at the edge until `is_night()`, and walks a path scored by distance to
    dwarves (a `path::search` goal filter that rejects tiles within 4 of an awake dwarf, retried once
    without). Escorts: one or two goblins `siege=false, sneaking=true` that follow and turn on whoever
    spots the thief.
  - *Wider theft and rustling (SIE-46):* targets include depot goods (`to_depot`) and pastured animals
    (a tame creature `led_by: Option<u32>` serde default follows the thief to the edge and is lost).
  - *Distraction thieves (SIE-47):* a partner lights a fire (fire.rs ignite) or batters a far door; the
    alert and the overseer's `ov_alert` go there; the thief strikes the other side.
  - *Baby-snatchers (SIE-08):* if B21 landed (CRE-04), N02 adds the *return*: a taken child is kept in
    `dread.taken: Vec<Taken { child: Dwarf, civ: u32, tick }>`; 4+ years later (so mostly in successive
    forts, P01) a siege from that civ includes them, grown, holding goblin values: parents and siblings
    recognise them (spotlight), and if a parent reaches them they may lay down arms (regard roll).
    Without B21 the snatching step here is the stand-in: a kidnapper skulker grabs a child.
  - *Bandits, cults and rackets (SIE-26):* bandit raids use FC8; the racket itself is P04 (ECO-27);
    here: small bandit raids on a young fort's surface stock. Cult: in a fort of 40+, stressed dwarves
    may join `Secret::Cultist { cult }` (a named cult of a dark power, ties to N03 pacts); cultists meet
    at night in an unused room (a `Case { kind: Missing }` if they abduct), and a sacrifice opens a
    murder case.
  - *Assassins and spies (SIE-27):* outside agents: a visitor (fake bard) with `PlotGoal::Assassinate`,
    reusing B04's plot runner (no new mechanics, only the arrival route).
  - *Poisoned water (SIE-21):* a saboteur (spy agent or outside sneak) reaches a well at night and
    fouls it: `dread.fouled.push((well, until = tick + 20 days, Some(culprit)))`, opens a
    `Case { kind: Fouling, at: well }`. Drinking (`Task::DrinkWater` arrival, game.rs:2716) from a
    fouled well: if D05 sickness exists, infect; else nausea (thirst not met, vomit, "drank foul water"
    stress, 5% chance of a lasting illness wound). Tracing: tracks (`Clue` +20 for whoever was seen
    near the well that night), a dwarf who noticed the smell and saw someone (+40), the saboteur's
    carried poison vial (an item `tainted` found on search when interrogated). Cleaning: a bucket brigade
    drains the cistern, or wait it out.
  - *Exile returns (SIE-28):* implemented once in B04 step 9; N02 only ticks it off.
  - *Kobold raids (SIE-48):* if B21's kobolds exist, add tricksters' habits: snares (a weak `CageTrap`
    placed on a path, traps.rs), poisonous vermin released in stores, a hoard of shiny loot in a
    cavern nook that can be raided back (an item cache with `label`). Else add `Species::Kobold`
    thieves that come in threes and taunt (gossip) when they escape.
- **Steps:**
  1. Sneaks before armies · combat.rs:maybe_siege · test `a_young_fort_meets_thieves_before_armies`.
  2. Thieves who plan + escorts · combat.rs:skulker_think (via H4 to new `src/game/shadows.rs`) · test
     `a_thief_waits_for_night_and_avoids_the_awake`.
  3. Rustling and depot theft · shadows.rs, creature.rs `led_by` · test `a_thief_leads_a_goat_away`.
  4. Distraction thieves · shadows.rs, fire.rs (call only) · test `a_fire_on_one_side_covers_a_theft_on_the_other`.
  5. Poisoned water and the trace · shadows.rs, game.rs DrinkWater arm (one call `self.drank_at_well`)
    · Done-when test `a_fouled_well_is_traced_back`.
  6. Baby-snatchers' return (or stand-in snatching) · shadows.rs · test `a_stolen_child_returns_with_the_enemy`.
  7. Bandits and cults · shadows.rs, FC8 · test `a_cult_meets_at_night_and_a_sacrifice_opens_a_case`.
  8. Outside assassins · plots.rs arrival only · test `a_false_bard_tries_to_poison_the_mayor`.
  9. Kobold habits · shadows.rs, traps.rs (snare) · test `kobolds_leave_a_snare_and_a_hoard`.
- **Overseer & screen:** `ov_dread`: guards the well (a burrow or a hatch/door on the well room for
  Cautious overseers), posts a night watch soldier at the depot when thefts happen twice, pens
  animals indoors after rustling, investigates sickness clusters (opens a Fouling case if 3+ dwarves
  sicken from one well within a day even before anyone saw the sabotage). Screen: "Someone has fouled
  the well in X" spotlight once noticed, theft gossip, the intrigue tab lists the case; chronicle.
- **Measure:** counters `thefts`, `animals_rustled`, `wells_fouled`, `fouling_traced`, `kidnapped`,
  `cultists`. Targets: first-year deaths fall vs M6 (sneaks before armies), fouling traced in 50%.
- **Risks / open questions:** distance-scored pathing must stay cheap: only for skulkers, at most once
  per 100 ticks per thief, and give up after 2 tries. Child kidnapping is dark; keep it off-screen and
  never show harm. Cults: user decision whether in scope for v0.9 (proposed: last step, can slip).
- **Parallel work:**
  - **Writes:** new `src/game/shadows.rs`; combat.rs `maybe_siege` (sneak branch) and `skulker_think`
    (moved to shadows.rs by H4); creature.rs `led_by` and Kobold row; game.rs DrinkWater arm (one call);
    traps.rs (snare placement fn); fire.rs (none, calls `ignite`).
  - **Reads / depends on:** FC2, FC4, FC8, B04 plot runner (`Plot`, `run_plot`), B21 if present, D05 if present.
  - **Hot spots:** combat.rs `maybe_siege` is also touched by B04 step 2 (one line): serialize N02
    step 1 after B04a or agree the order (B04 line first). game.rs DrinkWater arm (one line).
  - **Agent size:** one agent for steps 1-5 (thieves and water), a second for 6-9 after B04a.

### N03 Curses and relics  (effort: 5 sessions; risk: med)

- **Design:**
  - *Curse framework (MYT-37):* FC3, built in the foundation wave; N03 adds the non-night kinds
    (BadLuck: 5% job failure and trips; Wasting: slow fatigue; Haunted: a ghost follows; Oathbreaker:
    no prayer need met) and spreading (`spread: bool`: a cursed dwarf who sleeps in a dormitory may pass
    BadLuck on).
  - *Relic store:* **one** field on `Item`: `#[serde(default)] pub relic: Option<Box<Relic>>` with
    `Relic { power: Option<Power>, curse: Option<CurseKind>, heirloom_of: Option<u32>, awakened: bool,
    story: Vec<String> }`. Set for artifacts made in moods (psyche.rs:741), vault treasure
    (ruins.rs), dragon hoard skulls, and old heirlooms brought by migrants.
  - *Cursed artifacts (MYT-21):* 15% of vault artifacts and 3% of mood artifacts carry a curse that
    touches whoever owns or carries them or sleeps near them (`Trigger::NearItem`). Lift: destroy it
    (a smash job at a forge with magma, or throw it in magma) or carry it away (a mission to a site).
  - *Cursed sites (MYT-23):* `dread.site_curse: Option<SiteCurse { cause: String, lift: Lift,
    stage: u8 }>` rolled at embark for evil cells and for reclaimed ruins (P01). Effects: crops grow
    slower, ghosts rise sooner, bogeymen (N01). Lifting is a `MysteryKind::CursedSite` (mysteries.rs
    gets one enum arm and a `cursed_site_stage` in a new file): dreams point to the vault artifact; the
    choice is to carry it off (lose the wealth) or keep the curse.
  - *Artifact powers (MYT-22):* `Power::{WardBlades (armour +3 in strike_mods), EverFullCup (a drink that
    never empties), ThoughtHelm (wearer reveals Secrets within 3 tiles: a clue +40 to B04 cases),
    HealingTouch (wounds of whoever the holder doctors heal faster), Luck (strikes miss the holder 15%)}`.
    5% of artifacts, 20% of vault ones. Powers attract questers (B04) and envoys (B02).
  - *Relics and heirlooms (MYT-36):* migrants sometimes carry an heirloom (family weapon, ring)
    `heirloom_of: Some(line)`; when its holder earns an epithet (heroes.rs) it awakens with a power.
    Holy relics placed in a temple (C10) awaken after a festival there.
  - *Prophecies (MYT-32, shared with D11 WLD-08):* one `Prophecy { text, event: ProphecyEvent, by: u64,
    fulfilled: Option<bool> }` type in dread.rs. Made when an engraver (W18) carves "a scene" (2%) or a
    divination shrine (C10 MYT-24) answers. `ProphecyEvent::{BeastReturns(u32), Eclipse,
    HeroFalls(u32), SiegeBreaks, FortFalls}`. The scheduler biases its own rolls 50% toward fulfilling
    (e.g. `maybe_megabeast` picks the named beast), so prophecies come true often, not always.
  - *Places of power (MYT-30):* two new site feature bits (terrain::feat): `SACRED_SPRING` (a pool
    whose water heals the worthy: pious, no unpunished crimes) and `HOLY_GROVE` (animals in it are not
    hunted by wildlife; killing there defiles it). Laid out by world.rs feature placement; one tile set
    marked via a `World.sacred: Vec<Pos>` list (serde default).
  - *Defilement (MYT-29):* an atrocity at a sacred place or temple (murder, killing in the grove,
    toppling a temple statue, a corpse left in the spring) sets the tiles' biome evil bit (world.biomes
    `0x80`, already read by the renderer and Husk spawning), births a husk or two, and curses the
    culprit (`Oathbreaker`, lift: rebuild the statue or pray at a temple for a season).
  - *Demonic pacts (MYT-33):* a desperate dwarf (stress past the breaking point, a loved one dying, or a
    siege leader at the gate with few soldiers left) is offered a pact: help comes (the siege leader is
    struck dead, a dying spouse lives) and `Curse { kind: Pact { price } }` is laid with a due date a
    year later: the price is taken (their firstborn sickens, they die and rise as a ghost, or a demon
    walks out of the deep). The overseer can make the pact for the fort in a last stand (A06): Bold or
    Greedy only.
  - *Done-when path:* a temple statue requires `BuildingKind::Statue` (mason, stone) and a Temple zone
    (exists, `ZoneKind::Temple`). If C10/D08 has not added Statue, N03 adds it. psyche.rs:390 (tantrum
    topples a building) gets one call `self.on_toppled(d, bid, pos)` which, in a Temple zone or at a
    sacred place, defiles and curses.
- **Steps:**
  1. Non-night curse kinds and spreading · curses.rs · test `bad_luck_spreads_in_the_dormitory`.
  2. Relic store + cursed artifacts + lifting by destruction · items.rs (field), new
     `src/game/relics.rs`, psyche.rs and ruins.rs (one call each `self.maybe_relic(item)`) · test
     `a_cursed_artifact_sickens_its_owner_until_destroyed`.
  3. Artifact powers · relics.rs, strike_mods.rs arm · test `a_warding_helm_turns_blades`.
  4. Heirlooms and relics awakening · relics.rs, heroes.rs (one call on epithet) · test
     `an_heirloom_awakens_when_its_holder_becomes_a_hero`.
  5. Statue + temple desecration + defilement + Oathbreaker · buildings.rs (Statue if missing),
     psyche.rs (one call), new `src/game/sacred.rs` · Done-when test
     `toppling_a_temple_statue_brings_a_curse`.
  6. Places of power · terrain.rs feature bits, world.rs placement, sacred.rs · test
     `the_sacred_spring_heals_the_worthy_only`.
  7. Cursed sites as a mystery · mysteries.rs (one enum arm + dispatch line), sacred.rs · test
     `a_cursed_site_is_lifted_by_carrying_the_artifact_away`.
  8. Prophecies · dread.rs (type), detail.rs or wherever engraving finishes (one call), scheduler bias
    in combat.rs:maybe_megabeast (one line) · test `a_carved_prophecy_comes_true`.
  9. Demonic pacts · new `src/game/pacts.rs`, curses.rs `PactPrice` · test `a_pact_saves_the_fort_and_takes_its_price`.
- **Overseer & screen:** `ov_dread`: Cautious/Pious destroy a cursed artifact; Greedy/Vain keep it
  (and endure it); a Pious overseer builds temples to lift Oathbreaker curses and guards sacred places
  (no logging in the grove, a burrow around the spring); Bold/Greedy accept a pact in a last stand.
  Screen: relic pages in the look panel (story lines), curses on the unit sheet once known, defiled
  land drawn with the evil palette, prophecies in the legends "Lore" tab with fulfilled/failed marks,
  pact spotlight and its price as a chronicle title.
- **Measure:** counters `curses_laid`, `curses_lifted`, `relics_awakened`, `defilements`,
  `prophecies`, `prophecies_fulfilled`, `pacts`. Targets: a prophecy in 25% of forts and 40-60% of them
  fulfilled; pacts rare (≤5% of forts).
- **Risks / open questions:** relic powers can unbalance combat (WardBlades); cap one power per
  wearer. Defiling the map biome changes wildlife spawns: check `wildlife()` copes with evil columns
  mid-game. Pacts that kill children: keep the price abstract or adult. User decision: should the
  Done-when statue be a full Statue building (art value, admiration) or a temple fixture only?
- **Parallel work:**
  - **Writes:** new `relics.rs`, `sacred.rs`, `pacts.rs`; curses.rs (kinds); items.rs `Item.relic`;
    buildings.rs `Statue`; terrain.rs feature bits; world.rs `sacred` list; mysteries.rs (one arm).
  - **Reads / depends on:** FC3, FC5, H5, H6; C10 temples (optional), W18 engraving (exists), heroes.rs.
  - **Hot spots:** psyche.rs tantrum (one call), psyche.rs artifact creation (one call), ruins.rs (one
    call), heroes.rs (one call), mysteries.rs (one arm), combat.rs maybe_megabeast (one line, also
    touched by N01 step 1: N01 first).
  - **Agent size:** two agents: N03a relics/powers/heirlooms (2-4), N03b sacred/defilement/sites/
    prophecies/pacts (1, 5-9).

---

## Milestone 8: a world of forts (v1.0)

**Order and why.** P01 first: it builds the world file, the fort-as-site and the chain batch that
every later item is measured with (crown favour, outposts and routes carry over between forts). Then
P03 (missions, scouting, bandits on roads, outposts) because P05's own caravans and raiders and P02's
pleas for help are missions; P04 needs bandits (rackets) and coffers; P02 can run beside P03/P04
since it mostly touches nobles; P05 last (it needs B01 caravans on the map, P03 routes and bandits,
P04 coffers).

- Critical path: H3 missions + H9 run_fort → P01 (world file, write-back, advance, survivors, reclaim,
  chain batch) → P03 missions/bandits → P05 caravans/raiders.
- Parallel: P02 alongside P03; P04 after FC8 bandits.
- Dependencies on earlier milestones: **B01** (caravans and armies moving on the world map, roads),
  **B02** (diplomacy, envoys; tribute → offerings), **B03** (armies abroad, the crown's levy MIL-22:
  serving the crown NOB-31 is the same thing), **B17** (yearly world pass, wars in play, bandit camps
  WLD-21), **B18** (missions with aftermath; may already do H3), **B19** (migrants with a past and
  survivors of fallen forts SOC-17 = OUT-10: implement once, in P01), **B22** (megabeasts take fallen
  forts CRE-19 = occupant), **A06** (martial law OVR-07 = NOB-36), **B08** (rooms for rent ECO-51).

**Release check.**
- Done-when tests for each item; P01's is a chain test in `src/game/worldchain.rs`:
  `three_forts_in_one_world_and_the_third_reclaims_the_first` (short years, forced fall).
- Chain batch `--autoplay --chain 3 --runs 8 --years 3 --seed 5000 --out batches/m8-chain.json`:
  every fallen fort appears as a ruin site in the world record and in the next fort's legends; in
  chains where a fort had survivors, at least one returns as a migrant in 60%; `--reclaim first`
  makes the third fort reclaim the first in 8/8 chains and the reclaimed fort survives year 1 at least
  as often as a fresh one; ordinary `--runs 25 --years 3` stays within 10 points of M7.

### P01 Fallen forts live on  (effort: 6 sessions; risk: high)

- **Design:**
  - *Fallen forts enter the world (WLD-10):* at embark `Game::embark_with` calls
    `overworld.found_fort(name, civ, sx, sy, start_year)` → a `SiteKind::Fortress` site with our civ,
    `Game.site_id = Some(id)` (old saves: create it in `after_load`). On fall (app.rs:460 and
    `run_fort` end) `write_back(rec, Fate::Fell{..})`: `fort_fell` marks the site ruined, picks an
    `Occupant` from the annals (the last siege's civ if it was "the fortress fell" → that civ's
    `DarkFortress`-style occupation; a megabeast or cave horror that killed the last dwarf → `Beast/
    Horror`, ties to B22 CRE-19; else `Dead`), logs "X fell to the goblins of Y in year Z", and copies
    the fort's legends (`overworld.events` written during play) into the world record's overworld.
  - *Time passes between forts (WLD-25):* `Overworld::advance(years, seed)` (public wrapper over
    `simulate_year`, the year counter moves). Before the next embark 1-5 years pass (`--gap` in the
    batch, a prompt in the UI); occupied ruins may be retaken or abandoned by history (`advance` also
    rolls for each player ruin: 20%/yr the occupant moves on → `Empty`).
  - *What old forts leave behind (OUT-11):* `FortRecord.journal` (chronicle titles, overseer notes,
    last 20 legends) and the save file. In a reclaim, the old overseer's journal is an item
    (`ItemKind::Book` labelled "the journal of X", readable in look mode), records in the office, and
    items in odd corners stay where they lay; the vault stays opened/unopened as it was.
  - *Survivors come back as migrants (OUT-10, also B19 SOC-17):* who survives a fall: the dwarves on a
    mission when it fell (`realm.missions[..].members`, else `expedition.members`), dwarves `hidden`
    (N01), and (on a retire) all living dwarves. They go to `WorldRecord.wanderers` with
    "survived the fall of X" and a grudge (`relations` towards nobody; a thought kept as memory if C01
    long memories exist). `migrants()` in a later fort of the same civ draws up to two wanderers per
    wave (their age advanced by the gap), announced by name; dwarves who knew them (wanderers who
    arrive together) start with their old regard.
  - *Reclaim a fallen fort (OUT-09):* the world map shows player ruins; embarking on one (or the batch
    `--reclaim first`) loads its save and calls `Game::reclaim(old, ow, seed, years)`: keep `world`,
    `buildings`, `zones`, `items`, `constructions`, `vault`, `burrows`; dead dwarves stay in `dwarves`
    with `alive=false` (so unburied ones can rise as ghosts: family.rs ghosts_tick works unchanged)
    and their corpses rot to bones by the gap; food and drink rot away (items.rs value 0 → remove);
    creatures: remove all, then spawn the occupant (an army camp `siege=true` of 3-6 in the halls, the
    beast in the great hall, or nothing); a fresh 7-dwarf expedition with the usual wagon arrives at
    the old embark point (`settle_expedition`), new `fortress_name` is kept ("Reclaimed X" legend);
    `overseer`: a new one in the new style, with `base/top/rooms/layout/anchors` copied from the old
    one so its plan resumes (`Overseer::resume_from(old)`). If the old save is missing, the ruin is
    embarked fresh with "ruins" decoration (rubble, a few bones).
  - *Retire and unretire (OUT-32):* GameOver-like screen "Retire" (`R` on the Autoplay screen or the
    pause menu): `write_back(Fate::Retired)`; the save is kept. On `advance`, a retired fort gets an
    abstract life: each year `pop *= 1.1` capped, a siege roll against a defence score (soldiers,
    bridge, traps counted at retirement) that can turn it into a ruin (with the record's occupant).
    Unretire = load its save and apply the abstract years: age dwarves, kill some (old age, siege
    toll by roll), add migrants, append legends "while you were away".
  - *UI flow:* GameOver screen gains `n` "Found a new fort in this world": writes the fall into the
    world record (FC6), advances 1-5 years, opens `Mode::WorldMap` on the advanced overworld with the
    ruin marked and the wanderers waiting. Title gains "Continue a world" listing world files.
  - *Chain batch:* `--autoplay --chain K [--gap Y] [--reclaim first|any|never]`: `run_chain(seed,
    styles, years, k, gap, reclaim) -> ChainTrial { seed, forts: Vec<Trial>, ruins: usize,
    returned: usize, reclaimed: Option<usize> }`. Each fort: `run_fort` (H9) on an embark chosen with
    `default_site` excluding used cells (or the first ruin when reclaiming), `write_back`, `advance`.
    Fallen `Game`s are kept in memory for reclaim (or written to a temp dir under `batches/`). report.rs
    `chain_report`: survival per fort index, survivors returned, reclaim success, causes per fort index.
- **Steps:**
  1. Fort as a site + `found_fort`/`fort_fell` + `Overworld::advance` · overworld.rs, game.rs
     (`site_id`, embark call), ruins.rs:after_load · test `a_fallen_fort_becomes_a_ruin_in_the_world`.
  2. World file and write-back · new `src/world_record.rs`, saves.rs `write_world/read_world/list_worlds`
     · test `the_world_record_round_trips_and_old_saves_still_load`.
  3. Chain batch with time between forts · overseer.rs `run_fort` (H9), new `src/game/worldchain.rs`
     `run_chain`, main.rs args, report.rs `chain_report` · test `a_chain_plays_two_forts_and_years_pass`.
  4. Survivors as migrants · world_record.rs (wanderers), game.rs:migrants (one call
     `self.take_wanderers(&mut came)`) · test `survivors_of_a_fallen_fort_turn_up_in_the_next`.
  5. Reclaim · game.rs `reclaim` (in new `src/game/reclaim.rs`), overseer.rs `resume_from` · Done-when
     test `three_forts_in_one_world_and_the_third_reclaims_the_first`.
  6. What old forts leave behind (journal book, records) · reclaim.rs, items.rs (Book kind if absent) ·
     test `the_old_overseers_journal_lies_in_the_reclaimed_office`.
  7. UI flow: game-over "new fort in this world", world map ruins, title "Continue a world" · app.rs
     (GameOver keys, title menu entry), ui.rs world map glyph for player ruins · test with
     `App::with_game_for_tests`: after `n`, mode is WorldMap and the world record lists the fort.
  8. Retire and unretire · reclaim.rs (abstract years), app.rs (retire key) · test
     `a_retired_fort_grows_and_is_found_changed`.
- **Overseer & screen:** the overseer of a later fort reads the record: its `Sense::Realm`/`Dread`
  rises when the fort sits within 10 cells of a ruin with an army occupant (Fortify first); a
  Sentimental overseer prefers to reclaim (in the batch `--reclaim any` uses this: Sentimental or Bold
  reclaim, Cautious never); returning survivors are spotlighted ("Urist, who lived through the fall of
  Boltedhalls, has come"), and their grudge makes them volunteer for the squad. Legends gain a
  "Forts of this world" tab: each fort's founding, fall, occupant and reclaimer.
- **Measure:** chain batch numbers above; plus memory/time: a chain of 3 forts must take ≤3.3x one fort.
- **Risks / open questions:** `reclaim` is the riskiest code in M8: ids (`next_id`) must continue from
  the old game; caches (`reach_cache`, `water_cells`, path caches keyed on `dig_version`) must be
  reset; overseer `resume_from` might replan over existing rooms (test the overseer does not dig
  twice). Save size: the chain keeps 2-3 Games in memory (~ a few MB each, fine). The world record
  duplicates the overworld that each fort save also holds: a fort save loaded into a world is
  re-pointed to the world file's overworld (newer wins). Decision for the user: does "Continue a world"
  replace "Continue" (last save) or sit beside it? Proposed: beside.
- **Parallel work:**
  - **Writes:** new `src/world_record.rs`, `src/game/worldchain.rs`, `src/game/reclaim.rs`; overworld.rs
    (`advance`, `found_fort`, `fort_fell`, SiteKind unchanged); saves.rs (three fns); game.rs
    (`site_id`, `world_file` fields — added in H2 —, `embark_with` one call, `migrants` one call);
    overseer.rs (`run_fort` from H9, `resume_from`); main.rs (args); report.rs (`chain_report`); app.rs
    (GameOver keys, title entry); ui.rs (world map glyph, legends tab in ui/legends.rs after H7).
  - **Reads / depends on:** H2, H3, H7, H9. B19/B22 if present (reuse).
  - **Hot spots:** app.rs GameOver arm (app.rs:1071) and title menu; main.rs `autoplay_batch` (one
    branch for `--chain`); game.rs `migrants` (one call, after N01/B04's `migrant_secrets`).
  - **Agent size:** too big for one: P01a world (steps 1-3), P01b people and places (4-6) after P01a,
    P01c UI and retire (7-8) after P01a, parallel with P01b.

### P02 Crown and succession  (effort: 4 sessions; risk: med)

- **Design:**
  - *Noble deaths matter (NOB-08):* in the death path (`kill_*`/annals death note) for a dwarf with
    `noble` or a post: kin and other nobles get grief and anger (regard to the fort leader falls), the
    next migrant wave shrinks by a third for a year (word.rs already models "word keeps them away":
    add a `Want`-like reason "a noble died"), and a grand burial (Pious/Vain overseer) seals a servant
    in the tomb only if the dead noble was a tyrant (optional, flagged).
  - *Succession in play (NOB-32):* `realm.claims: Vec<Claim { title: Noble, claimants: Vec<u32>,
    since: u64 }>`. When a titled noble dies: claimants = spouse, eldest child, the highest-regard
    friend, the sheriff if Count+. One claimant clear (regard majority) → crowned; else a contested
    claim: factions form (C03 feuds if present, else regard swings), a season later a vote or a
    usurper (a claimant with high fight skill and temper may "take it": a crime case). Positions
    (manager etc.) vacated by death are refilled by the overseer (exists in `ov_posts`).
  - *Offerings to the ruler (NOB-44):* the overseer (or player, a trade-screen "offer" key) gives items
    to the civ's caravan as gifts; `realm.favour += value/100`. Favour shortens the wait for titles and
    makes the monarch's demands gentler.
  - *Monarch arrives (NOB-30):* when `earned_title() == Duke`, average stress low, wealth ≥ 200k and
    `favour ≥ 20`, the civ ruler (`Civ.ruler`, an overworld `Figure`) moves in with 3-5 entourage
    (a new `Noble::Monarch` and `Noble::Retainer`; the figure becomes a `Dwarf` with
    `figure: Some(id)` — one field already planned in H2 `Dwarf.figure` or stored in `realm.monarch`
    mapping). The capital moves: `Civ.capital = our site_id` (needs P01's site). Demands: a throne room
    (an Office of room value ≥ 5,000 with a gold/steel chair), a royal tomb, mandates more often.
    Satisfied: a "Mountainhome" milestone, sieges against the capital are rarer (the realm defends),
    the overseer gains a `Realm` sense.
  - *Serving the crown (NOB-31):* if B03 MIL-22 "the crown's levy" exists, reuse it; else each year
    the monarch or the civ calls 2-4 soldiers to a war in the world (B17): they leave as a mission and
    return a season later as veterans, or not (a `battle` result from the overworld war).
  - *Food riots (NOB-18):* when hunger sense ≥ 0.6 and a noble's room/office holds food or the noble
    eats prepared meals while others starve, 3+ hungry dwarves riot: tantrum behaviour in a group,
    loot the food stockpile nearest the noble's quarters, vandalise. The sheriff/guards (soldiers) can
    break it up (fights with fists); the overseer may ration (`Revel` down).
  - *Martial law (NOB-36):* if A06 has OVR-07 reuse it; else when unpunished crimes ≥ 6 or a riot
    happened twice in a season, a noble (temper) declares martial law: every adult drafted into
    squads for a season, stress +, crimes punished doubly. Everyone hates it.
  - *Pleas for help (NOB-35):* when B17's world turn shows a friendly site besieged (or a friendly
    civ at war within 20 cells), an envoy/messenger asks for aid. The aid itself is P03's `Aid`
    mission (OUT-14): implement the mission once there, the plea here.
- **Steps:**
  1. Noble deaths matter · new `src/game/crown.rs`, death path one call `self.noble_died(d)` · test
     `a_barons_death_angers_kin_and_chills_migration`.
  2. Succession and claims · crown.rs, positions.rs (one call from `titles_daily`) · test
     `a_contested_claim_splits_the_fort_and_resolves`.
  3. Offerings · crown.rs, trade.rs (gift settlement) · test `gifts_raise_the_crowns_favour`.
  4. Food riots · crown.rs, psyche.rs tantrum reuse · test `starving_dwarves_riot_at_the_full_noble_larder`.
  5. Martial law (or reuse A06) · crown.rs, military.rs (draft-all) · test `martial_law_drafts_everyone_and_sours_moods`.
  6. Monarch arrives and demands a throne room · crown.rs, dwarf.rs `Noble` (append variants),
    positions.rs (`TITLES` unchanged, a new check) · Done-when test
    `a_happy_rich_fort_is_visited_by_the_monarch_who_demands_a_throne_room`.
  7. Serving the crown (or reuse B03) · crown.rs + a `MissionKind::Levy` · test `levied_soldiers_return_as_veterans_or_not`.
  8. Pleas for help (plea only) · crown.rs · test `a_besieged_ally_begs_for_aid`.
- **Overseer & screen:** `ov_realm`: Vain overseers chase the monarch (offerings, throne room as an
  `Architect` grand work: add `RoomUse::ThroneRoom` to the room wants), Cautious ones ration food early
  to avoid riots, Bold ones answer pleas, Stubborn ones refuse martial law. Sense `Realm` rises with
  favour targets and a pending demand. Screen: nobles screen shows the line of succession and claims;
  the monarch's arrival is a spotlight and a chronicle title; riots get a spotlight.
- **Measure:** counters `noble_deaths`, `successions`, `contested`, `riots`, `martial_law`,
  `monarch_arrived`. Targets: monarch arrives in ≥50% of Glory forts in a 5-year batch; riots in
  ≤15% of forts; martial law rare.
- **Risks / open questions:** the monarch's figure ties the overworld to a `Dwarf`: if the fort falls
  with the monarch, the civ needs a new ruler (`crown()` in overworld.rs — make it public). Riots plus
  martial law plus tantrums can cascade; cap one riot per season. Decision: should the player be able
  to refuse the monarch (NOB-11-ish)? Proposed: no, but a Stubborn overseer can stall demands.
- **Parallel work:**
  - **Writes:** new `src/game/crown.rs`; dwarf.rs `Noble` (append Monarch, Retainer + `title()`
    arms); positions.rs `titles_daily` (one call); trade.rs (gift hook); military.rs (draft-all fn);
    overworld.rs `crown` made `pub`.
  - **Reads / depends on:** FC7 (`realm`), P01 step 1 (`site_id`) for the capital move, P03 missions for
    levy/aid (or B03/B18), B17 for pleas.
  - **Hot spots:** death path (single call), dwarf.rs `Noble` enum (append only), positions.rs one call.
  - **Agent size:** one agent for steps 1-6; steps 7-8 after P03a.

### P03 Roads and outposts  (effort: 4 sessions; risk: med)

- **Design:**
  - *Scouting missions (OUT-23):* `realm.explored` bitset (empty = all known, so old saves and players
    who don't care see everything); with `--fog` / a world option new forts start knowing only cells
    within 8 of the site plus their civ's sites. `MissionKind::Scout` (one dwarf, fast, risky by
    savagery): reveals cells in a radius around a target; discoveries get names (fords, peaks — the
    region namer exists) and become destinations (ruins, lairs, P01 player ruins).
  - *Messengers between sites (OUT-31):* news delay: missions and outposts report by messenger
    (`MissionKind::Messenger(text)` arriving after `dist/4` days); a messenger can be waylaid by
    bandits (the news never comes: "no word from the outpost"). B02/B24 messengers reuse this.
  - *Bandits on the road (OUT-08):* FC8 camps; any mission whose route passes within 3 cells of a camp
    rolls an ambush (strength vs the camp's): lose cargo, members wounded or killed, or the bandits
    beaten (loot). Camps can be hunted (an expedition goal `Goal::Bandits(site)`), paid (P04 racket),
    or ignored.
  - *Outposts and attached hamlets (OUT-30):* `Outpost { site: u32, name, kind: OutpostKind::{Mine,
    Logging, Farm, Hamlet}, workers: Vec<Dwarf>, stock: Vec<Item>, defence: i32, next: u64 }` founded by
    sending 2-5 dwarves to a cell within 10 (a mission that does not come back; they live off-map like
    expedition members). Each season it produces goods by kind and worker skills (ore, logs, food) and
    sends them home as a mini-caravan to the depot (a `Mission` with `cargo`). Raided by bandits or a
    hostile civ (abstract battle with `squad_strength`-like score); a raided outpost loses goods,
    workers die (death notes "fell defending the outpost"), it can fall (becomes a ruin site). Attached
    hamlets (B03 OUT-30 mentions hill dwarves): a nearby Hamlet of our civ may "attach" and send food
    and migrants (overlap with P05 ECO-33: build once here as `OutpostKind::Hamlet`).
  - *Aid missions (OUT-14):* `MissionKind::Aid(site)` answers P02's plea: a squad goes; success raises
    the asking civ's favour and our standing (fewer sieges from their enemies? no: better caravans and
    migrants); refusing lowers it.
- **Steps:**
  1. Missions refactor (if H3 not done by B18) and routes on the overworld (straight-line cell paths,
     `route: Vec<(i32,i32)>`) · expeditions.rs → `missions.rs` · test `old_expedition_saves_become_missions`.
  2. Scouting and fog · new `src/game/roads.rs`, ui.rs world-map fog draw (in ui/realm.rs) · test
     `a_scout_reveals_and_names_a_ford`.
  3. Bandit camps (FC8) and ambushes on routes · overworld.rs, roads.rs · test `a_caravan_route_past_a_camp_is_ambushed`.
  4. Messengers · roads.rs · test `news_from_a_mission_arrives_late_or_not_at_all`.
  5. Outposts and deliveries · new `src/game/outposts.rs` · test `an_outpost_sends_ore_home_each_season`.
  6. Outpost raids · outposts.rs · Done-when test `an_outpost_sends_goods_home_and_is_raided_by_bandits`.
  7. Aid missions · roads.rs · test `answering_a_plea_raises_their_favour`.
- **Overseer & screen:** new ambition behaviour inside `Venture` (no new ambition): Venture now also
  founds an outpost when ore/wood troubles (`Lack::Ore`, `Lack::Wood`) persist and the pop ≥ 20;
  Cautious overseers hunt bandit camps before sending caravans; Bold ones answer aid pleas. Screen:
  missions on the world map (B01 drawing if present, else a list), outposts listed on a Realm tab
  (ui/realm.rs) with stock and defence, messages "Word from the outpost of X: ...".
- **Measure:** counters `outposts`, `outpost_deliveries`, `outposts_raided`, `ambushes`, `scouted`.
  Targets: an outpost in 30% of 5-year forts that reach 20 dwarves; outposts fall in ≤30% of those.
- **Risks / open questions:** off-map dwarves must not count as "alive in the fort" for survival but
  must count for P01 survivors; define `alive_count()` (on-map) vs `living_people()` (all). Outposts
  drain the fort's workforce (the overseer must not found one during a siege or famine).
- **Parallel work:**
  - **Writes:** `missions.rs` (from expeditions.rs), new `roads.rs`, `outposts.rs`, `ui/realm.rs`;
    overworld.rs (BanditCamp kind, camps fns, region naming made `pub`).
  - **Reads / depends on:** H3, FC7, FC8, P01 step 1 (`advance` spawns camps), B01 world-map drawing.
  - **Hot spots:** overworld.rs `SiteKind` (append + `name/glyph` arms; also touched by P01: P01 first),
    ambitions.rs Venture arm (one link line), overseer.rs (via `ov_realm` only).
  - **Agent size:** P03a missions/scouting/bandits/messengers (1-4), P03b outposts/aid (5-7).

### P04 Coin and contract  (effort: 4 sessions; risk: med-high)

- **Design:**
  - *Money:* `realm.coffers: i64`, an abstract treasury (DF dropped coin items; see decision below).
    Income: trade profit (trade.rs settlement adds the value difference the broker wins), outpost goods
    sold, rents. Spending: contracts, wages, debts.
  - *Mercenaries and monster hunters for hire (ECO-12):* a company arrives (visitor route) offering a
    contract: `Contract { kind: ContractKind::{Guard, Hunt(creature)}, party: String, members:
    Vec<u32>, paid: i64, owed: i64, due: u64, until: u64 }`. Members are **`Dwarf` entries** flagged
    `hireling: Some(contract)` (a Box in `Secret`-like extension or a field in `Dwarf` added in H2)
    put into a squad, so they use the existing dwarf military AI (creatures can't fight creatures).
    Monster hunters (F07) reuse it with `Hunt`. They can fail, or cheat (leave with the first half,
    ECO-25).
  - *Contracts with part payment (ECO-25):* half up front from coffers, half on completion; a
    disreputable company (`party_honour`) may vanish after the first half (gossip, legend).
  - *Patrons fund works (ECO-26):* a rich noble pays coffers (from their room value as "wealth") to
    raise a monument/hall in their name: megaprojects.rs `Monument` with `patron: Option<u32>`; the
    legend names them.
  - *Protection rackets (ECO-27):* a bandit camp (FC8) within 10 cells demands a cut (10%) of each
    caravan's trade; pay (coffers down) or refuse (a bandit raid via `lay_siege_of` with the bandit
    Neighbor, and caravans ambushed by P03).
  - *Rooms for rent (ECO-51):* tavern guests (visitors) pay for a bedroom zone flagged `for_rent`; B08
    has the tavern and inns: implement only the rent tick here, shared.
  - *Loans and creditors (ECO-52):* borrow from the civ's merchants (caravan screen) with interest;
    unpaid by the due date → creditor civ turns hostile (`enemies.push` on overworld civs if not our
    own; if our own civ, the crown's favour collapses and a debt collector party comes to seize goods).
  - *Coins and wages (ECO-53):* optional: wages paid monthly from coffers to soldiers and hirelings;
    unpaid hirelings desert (Done-when). Citizen wages are off by default.
- **Steps:**
  1. Coffers and income from trade · new `src/game/coin.rs`, trade.rs (one call) · test `profit_fills_the_coffers`.
  2. Mercenary companies as hirelings with a guard contract · coin.rs, visitors.rs (arrival arm),
     military.rs (squad of hirelings) · test `a_hired_company_guards_the_gate`.
  3. Part payment and wages, desertion · coin.rs · Done-when test `a_mercenary_company_deserts_when_its_pay_runs_out`.
  4. Monster hunters on contract (reuse F07) and cheats · coin.rs · test `a_hunter_takes_half_and_vanishes`.
  5. Patrons · coin.rs, megaprojects.rs (patron field) · test `a_rich_baron_pays_for_a_monument_in_his_name`.
  6. Rackets · coin.rs, FC8 · test `refusing_the_racket_brings_bandits`.
  7. Loans and creditors · coin.rs, trade.rs (borrow on the trade screen) · test `an_unpaid_loan_makes_an_enemy`.
  8. Rooms for rent (with B08) · coin.rs, rooms.rs `for_rent` · test `a_guest_pays_rent_for_a_room`.
- **Overseer & screen:** `ov_realm`: Greedy overseers hoard coffers and refuse rackets (more raids),
  Cautious ones hire guards before the first siege if coffers allow, Bold ones hire monster hunters
  against a known horror, Vain ones accept patrons. Coffers show on the status bar (☼ is wealth; use a
  coin glyph). Contracts on the Realm tab. Desertion is a spotlight.
- **Measure:** counters `contracts`, `deserted`, `cheated`, `racket_paid`, `racket_refused_raids`,
  `loans_defaulted`. Target: hired guards cut first-siege deaths in forts that hire (compare subsets).
- **Risks / open questions:** money is a new economy that can trivialise defence (hire your way out);
  price contracts by `siege_strength()` so they cost a real share of a year's trade. Hirelings as
  dwarves: they must not marry, mood, become nobles, or count as citizens for migration; a
  `hireling` check in the few places (families_daily, strange moods, titles) is the price.
  **Decision for the user:** coin items or abstract coffers? Proposed: abstract (DF dropped coins).
- **Parallel work:**
  - **Writes:** new `src/game/coin.rs`; trade.rs (income + loan hooks), visitors.rs (company arrival),
    military.rs (hireling squad), megaprojects.rs (patron), rooms.rs (`for_rent`), family.rs/psyche.rs/
    positions.rs (`hireling` guard, one line each).
  - **Reads / depends on:** FC7, FC8, P03a ambushes for rackets, B08 for rent (optional), F07 hunters.
  - **Hot spots:** trade.rs settlement (also hooked by B04 traitor skim: same function, two separate
    one-line calls), `Dwarf.hireling` (add in H2 to avoid a Dwarf conflict).
  - **Agent size:** one agent for 1-5, a second for 6-8 after P03a.

### P05 Trade routes of the world  (effort: 4 sessions; risk: med)

- **Design:**
  - *Caravans on the world map (ECO-32):* if B01 drew moving caravans, reuse; else a caravan becomes a
    `Mission`-like traveller in `realm.routes`: `Route { from_site, to_site, cells, kind, eta }`
    drawn on the world map, arriving when `eta` passes (replacing the bare seasonal spawn in
    `caravan_season`).
  - *Caravan raiders (ECO-28):* travellers on routes pass bandit camps → ambush roll (P03 step 3
    reused); a raided caravan arrives smaller or not at all ("the human caravan was waylaid on the
    road"). A bribed merchant (B04 plot) leaks the cargo of our own caravans.
  - *Fairs (ECO-30):* once a year (late summer), if 2+ friendly civs are within 30, a fair: several
    caravans at once (`caravan: Option<Caravan>` → `caravans: Vec<Caravan>` with a serde shim like
    H3), better prices (`price_mult 0.9`), a festival (festivals.rs, one new kind) with visitors.
  - *Your own caravans (ECO-31):* `MissionKind::Caravan(site)`: pick goods (overseer: surplus crafts),
    a wagon and 2 dwarves go to a friendly site; return after `days()` with goods chosen by that
    site's stock or coffers; raidable on the road.
  - *Hill and deep dwarf partners (ECO-33):* the attached hamlet from P03 (`OutpostKind::Hamlet`) plus
    a deep-dwarf site placed in a cavern layer of worlds generated after this (an overworld site with
    `deep: bool`); deep dwarves arrive through the caverns with food and fungus goods, and send
    migrants.
  - *World economy (ECO-54):* sites get `stock: [u16; 8]` over goods classes (food, drink, wood,
    stone, metal, cloth, crafts, gems) produced by biome/race and consumed by pop in
    `Overworld::advance`; caravans draw goods from their home site's stock; our sales are added to the
    buyer site's stock and show up in other civs' caravans later ("dwarven crafts from X"). Prices
    (B24 ECO-23 local prices) read scarcity. Keep it coarse: one pass a year.
- **Steps:**
  1. Routes and caravans as travellers · new `src/game/routes.rs`, trade.rs `caravan_season` (one
     branch) · test `a_caravan_is_seen_on_the_road_before_it_arrives`.
  2. Raiders on routes · routes.rs (reusing P03) · test `a_waylaid_caravan_never_comes`.
  3. Your own caravans · routes.rs, missions.rs · test `our_wagon_sells_crafts_and_brings_back_steel`.
  4. Fairs (`caravans` vec shim) · trade.rs, festivals.rs (fair kind), ui trade screen picks a caravan ·
     test `a_fair_brings_three_caravans_at_once`.
  5. Hill and deep dwarf partners · routes.rs, overworld.rs (deep site), outposts.rs (Hamlet) · test
     `deep_dwarves_come_up_through_the_caverns_to_trade`.
  6. World economy and a new route to a distant site · overworld.rs `stock`, `advance` · Done-when test
     `a_route_opened_to_a_distant_site_brings_a_new_kind_of_caravan`.
- **Overseer & screen:** `Prosper` ambition sends our own caravans when surplus crafts > 30 and
  opens a route to the nearest friendly site not yet traded with; Greedy overseers chase fairs;
  Cautious ones skip routes past bandit camps. Screen: routes and travellers on the world map, a
  "Trade" section on the Realm tab with routes, last caravans and site prices.
- **Measure:** counters `caravans_arrived`, `caravans_waylaid`, `own_caravans`, `fairs`, `routes`.
  Target: no fort loses all caravans to raiders over 3 years in more than 10% of forts.
- **Risks / open questions:** `caravan` → `caravans` touches trade.rs and the trade UI widely: do it as
  a separate shim commit first. The economy pass must stay fast in `advance` (sites × 8 numbers).
  World economy is ★ fun: user decision whether to ship step 6 minimal (stock only on sites we
  traded with).
- **Parallel work:**
  - **Writes:** new `src/game/routes.rs`; trade.rs (`caravans` shim, `caravan_season` branch);
    festivals.rs (fair); overworld.rs (`Site.stock`, deep sites); ui trade screen.
  - **Reads / depends on:** FC7, FC8, H3, P03a (ambush roll `ambush(route) -> AmbushResult`), P03b
    (Hamlet), P04 step 1 (coffers), B01/B24 if present.
  - **Hot spots:** trade.rs (shim commit isolated), overworld.rs `Site` (append serde-default field).
  - **Agent size:** one agent for 1-3, a second for 4-6.

---

## Later

**B05 Artifact politics (M; CRF-09, CRF-04, CRF-05, CRF-07).** Builds directly on B04's questers and
plots and N03's relic store: an artifact's `Relic.story` gains a `whereabouts` log, and the world
record (FC6) keeps every artifact made in any fort, so invaders can arrive *for* a named artifact
(siege goal `PlotGoal::StealArtifact` carried by `lay_siege_of`), envoys (B02) can demand it, rumours
(B17 news) say which site holds a stolen one, and a squad mission (`MissionKind::Expedition(Goal::
Recover(item))`) can fetch it back. The overseer's Greedy/Vain traits refuse demands; Sentimental ones
mount recoveries. Effort ~3 sessions once B04 and P03 exist; measure "artifacts lost and recovered".

**B08 Taverns, temples, libraries (M).** The fort's public rooms as places that do things:
performances (F07 performers), guests renting rooms (P04's rent tick), prayer at temples (C10's
gods), and a library where scholars write books (`ItemKind::Book`, introduced by P01's journal) whose
topics come from the fort's legends and N03's prophecies. Mostly room-zone logic in rooms.rs and
visitors.rs plus the overseer's `RoomUse` wants; ~3 sessions. It is best done right after C09/C10
since most of its value is in reusing them.

**B09 Guilds and sects (M; SOC-33, SOC-34, SOC-35, SOC-54, SOC-55).** Skilled dwarves of one craft
(skill ≥ 10) form a guild that demands a guildhall (a room demand like `Demand`), takes sides in
feuds (C03 factions), and grants perks (faster work in their hall, cheaper hires). Sects are the
religious version (C10 gods, N02 cults as the dark mirror: share the `Cultist` membership type as a
general `Membership { group: u32 }`). The overseer answers guild demands like noble demands; refusing
makes the guild strike. ~3 sessions after C03/C10.

**B14 Myths and magic (XL; MYT-58, MYT-46, MYT-45, MYT-47, MYT-48, MYT-51, MYT-56).** World generation
gains a creation myth and a pantheon with powers (C10's gods become real actors), wizards and covens
as overworld figures, and magical invasions. The pieces M7 builds are its spine: N01's
`wards_work` and banes become per-world myth rules, N03's curses/powers become spells, B04's villain
networks get magical masters, and the overseer's Pious trait answers miracles. Effort: a milestone of
its own (8-12 sessions); plan it after v1.0 with its own Done-when (a coven's curse lands on a fort
and the fort learns the counter-rite).

**X01 Lost places below (L; DIG-16, DIG-23, DIG-24, DIG-25, DIG-30, DIG-29, DIG-37).** More things
like F08's vault in the rock: crypts of heroes (whose relics are N03 heirlooms and whose dead rise
with B23's dormant dead), ruins that tell their fall in engravings, kobold warrens (N02's kobold
hoards), goblin pits connected to the caverns. Each is a placement in world.rs plus a staged discovery
in the mysteries pattern; ruins.rs's `place_vault` generalises into `place_lost_place(kind)`. ~4
sessions; the overseer's Delve ambition targets them.

**X02 Wear, rust and fashion (M; CRF-30, CRF-11, CRF-47, CRF-48, CRF-27, CRF-39).** `Item.wear` and
`WORN` exist (items.rs:392); extend wear to armour and weapons in combat, rust for iron in wet places
(water tiles nearby), dyes (a dyer's job and a `colour` on cloth), and fashion: a year's favoured
colour that vain dwarves want (a need). Small per step, ~3 sessions; the overseer re-equips worn gear
(a `Lack` for armour) and the batch watches that worn gear doesn't lose sieges.

**X03 Clearer orders (M; UI-29, UI-11, UI-12, UI-20, UI-26, UI-27).** Why-can't notices (a job that
can't start says why: no path, no material, forbidden), stuck-path markers on the map, job-done
notices, and a settings/difficulty screen. Much of this reads data the troubles system already keeps
(`troubles::Lack`, `Lack::from_desc`): surface it in a "Why" pane. ~3 sessions; no balance risk;
good onboarding value for v1.0 and could be pulled earlier.

**X04 The living wild (L; CRE-17, CRE-16, CRE-24, CRE-27, CRE-34).** Finite wildlife populations per
world cell (kept in the overworld, so P01's successive forts find a hunted-out land), the wild's anger
at felling (elves' complaints from B02, then ents/animal-people raids), and guardian animal people.
Uses N03's places of power (the holy grove) and B20's populations; ~4 sessions. The overseer's
Provision ambition must log less in angry lands (a Cautious trait reaction).

---

## Work packages and waves

Effort is in agent sessions. Packages in the same wave edit disjoint functions; where two touch the
same file it is noted with the exact function so the second can rebase trivially.

### Wave 0 (serial, one agent, ~2 sessions): foundations

| Package | Contents | Writes |
|---|---|---|
| **W0-hooks** | H1 system hooks, H2 containers + Dwarf fields (`curse`, `secret`, `hireling`) + Task variants (`Feed`, `Interrogate`), H4 think dispatch, H5 strike hook, H6 overseer hook + `Sense::Dread/Realm`, H8 Trial tallies + empty report sections, H9 `run_fort` | game.rs (step, struct, Task), dwarf.rs, combat.rs (`creature_think`, `strike`), overseer.rs (`ov_daily`, `run_trial`), ambitions.rs, report.rs, new dread.rs/realm.rs/strike_mods.rs/ov_dread.rs/ov_realm.rs |
| **W0-ui** | H7 UI split (nobles, legends into src/ui/, empty intrigue/realm tabs) | ui.rs, new src/ui/*.rs |
| **W0-missions** | H3 expedition → missions shim (skip if B18 did it) | expeditions.rs → missions.rs, game.rs field shim, ruins.rs `after_load` |

W0-ui and W0-missions can run in parallel with W0-hooks only if W0-hooks lands its game.rs struct
changes first; simplest is serial. Verification: `cargo test --release` green, a 5-fort smoke batch
identical in outcome distribution to the M6 release (no behaviour change).

### Wave 1 (parallel): M7 core foundations + P01 world

| Package | Items/steps | Sessions | Notes |
|---|---|---|---|
| **M7-F** | FC1 clock, FC2 cases, FC3 curses (framework only), FC4 secrets/plots types | 2 | new files only + justice.rs `CrimeKind` variants |
| **P01a** | P01 steps 1-3 (site, world file, advance, chain batch) | 2 | overworld.rs, saves.rs, new world_record.rs, worldchain.rs, main.rs |

Disjoint: M7-F never touches overworld.rs/saves.rs/main.rs. Coordinator batch after wave 1: the chain
smoke `--chain 2 --runs 4 --years 1`.

### Wave 2 (parallel, needs wave 1)

| Package | Items/steps | Sessions | Shared-file notes |
|---|---|---|---|
| **N01a** | omens, banes, werebeasts, bogeymen (N01 1-3, 6) | 2 | combat.rs `maybe_megabeast` one line; necromancy.rs one line |
| **N01b** | vampire, hunt, sensing, wards (N01 4, 5, 7, 8) | 2 | game.rs `migrants` via `migrant_secrets` (created in W0); family.rs `new_year`; needs.rs |
| **B04a** | questers, spies, double agents, villain networks (B04 1, 2, 4, 6) | 2 | visitors.rs; combat.rs `maybe_siege` one line; `migrant_secrets` body shared with N01b → **B04a adds to `migrant_secrets` after N01b merges, or each adds a separate inner call** (`self.maybe_vampire(came); self.maybe_spy(came)`) — set up both calls in W0 |
| **N03a** | relics, powers, heirlooms (N03 2-4) | 2 | items.rs `relic`; psyche.rs artifact (one call); ruins.rs (one call); heroes.rs (one call); strike_mods.rs separate arm |
| **P01b** | survivors, reclaim, leftovers (P01 4-6) | 2 | game.rs `migrants` (one call `take_wanderers`, different line) |
| **P01c** | UI flow, retire (P01 7-8) | 2 | app.rs GameOver arm, title menu; ui/legends.rs |

Can run together: all six (they touch different functions). Must serialize inside a file: strike_mods
arms (N01a then N03a: trivial rebase), `maybe_megabeast` (N01a only in this wave).
Coordinator batch after wave 2: `--runs 25 --years 3 --seed 5000` vs M6 and `--chain 3 --runs 8`.

### Wave 3 (parallel, needs wave 2)

| Package | Items/steps | Sessions | Shared-file notes |
|---|---|---|---|
| **B04b** | interrogation, traitor, assassination (B04 3, 5, 7) | 2 | justice.rs dispatch; trade.rs one call; orders.rs one hook; troubles.rs `ov_seen_stock` |
| **N02a** | sneaks first, planning thieves, rustling, distraction, poisoned water (N02 1-5) | 2 | combat.rs `maybe_siege` (sneak branch; after B04a's line); game.rs DrinkWater arm |
| **N03b** | curses kinds, desecration, places of power, cursed sites, prophecies, pacts (N03 1, 5-9) | 3 | psyche.rs tantrum one call; mysteries.rs one arm; combat.rs `maybe_megabeast` bias (after N01a); terrain.rs/world.rs features |
| **P03a** | missions/routes, scouting, bandits, messengers (P03 1-4) | 2 | overworld.rs `SiteKind` append (after P01a) |
| **P02a** | noble deaths, succession, offerings, riots, martial law, monarch (P02 1-6) | 2 | dwarf.rs `Noble` append; positions.rs one call; trade.rs gift hook (different function from B04b's skim) |

Coordinator batch after wave 3: full 25×3 and 25×5 (monarch needs years), chain 3×8.

### Wave 4 (parallel, needs wave 3)

| Package | Items/steps | Sessions |
|---|---|---|
| **B04c** | intrigue screen, exile returns (B04 8-9) | 1-2 |
| **N02b** | snatchers' return, bandits and cults, outside assassins, kobolds (N02 6-9) | 2 |
| **P03b** | outposts, raids, aid (P03 5-7) | 2 |
| **P04a** | coffers, mercenaries, wages and desertion, hunters, patrons (P04 1-5) | 2 |
| **P02b** | serving the crown, pleas (P02 7-8) | 1 |

### Wave 5 (parallel, needs wave 4)

| Package | Items/steps | Sessions |
|---|---|---|
| **P04b** | rackets, loans, rent (P04 6-8) | 2 |
| **P05a** | routes as travellers, raiders, own caravans (P05 1-3) | 2 |
| **P05b** | fairs (`caravans` shim first), partners, world economy (P05 4-6) — after P05a | 2 |

M7 release after wave 4's B04c/N02b; M8 release after wave 5.

## Verification per package

Every package: its listed tests added in the module's `#[cfg(test)] mod tests`, the Done-when test if
it owns one, `cargo test --release` green, and an old-save load test where it adds serde fields
(`serde_json::from_str` of a save written before the field, e.g. by deleting the field from a fresh
save's JSON in the test). No package runs a 25-fort batch; the coordinator runs, once per wave:

- after wave 1: chain smoke (`--chain 2 --runs 4 --years 1`), confirms write-back and advance;
- after wave 2: `--runs 25 --years 3 --seed 5000 --out batches/w2.json`, `--compare` with M6 (survival
  within 10 points; dread section shows vampires, banes, werebeasts) and `--chain 3 --runs 8` (survivors
  return, reclaim works);
- after wave 3: the same plus `--years 5` for the monarch and prophecies;
- after wave 4: M7 release check numbers;
- after wave 5: M8 release check numbers.
Packages that change siege or combat balance (N01a banes, N02a sneaks-first, B04a spy-guided sieges,
P04a mercenaries) are flagged for the coordinator to read the siege section of the batch report first.

## Cross-milestone notes

- **One "who did it" store (FC2).** C07's witnesses, N01's vampire hunt, B04's spies and traitors,
  N02's fouled wells and N03's desecration are the same mechanic: a hidden culprit, clues with weights,
  an accusation that can be wrong. Build it once (in C07 if M3 comes first, else in M7-F) and let the
  intrigue screen show every case.
- **The expedition stash is a general tool.** Removing `Dwarf`s from `game.dwarves` into a holder
  (expeditions.rs) already works for saving, relations and return. Werebeast forms, outposts, levies,
  own caravans and fallen-fort survivors all reuse it; generalise it once as `Mission` (H3) and add
  `living_people()` vs `alive_count()` so off-map dwarves are neither lost nor counted as present.
- **Threat scheduler with lead time.** Omens (N01), prophecies (N03, D11), scouts (B01) and spies
  (B04) all want threats decided before they arrive. Moving `maybe_siege/maybe_megabeast/maybe_undead/
  maybe_cave_horror` behind one `dread.pending` queue (N01 step 1) gives every one of them a hook.
- **Overworld must become mutable and public in play.** M8 needs `advance`, `found_fort`,
  `fort_fell`, `crown`, bandit camps and site stock; B17 (M4) needs the same yearly pass. Do one
  `pub fn advance` and one `world_record` file, and treat the overworld inside a fort save as a copy of
  the world file's.
- **Duplicates to build once:** exile returns (NOB-33 = SIE-28, in B04), secret vampire (MYT-04 =
  NOB-24, in N01), assassins (NOB-27 ≈ SIE-27, B04 runner), pleas for help (NOB-35) vs aid missions
  (OUT-14) (plea in P02, mission in P03), survivors return (OUT-10 = SOC-17 of B19, in P01), attached
  hamlets (OUT-30) vs hill dwarf partners (ECO-33) (in P03), prophecies (MYT-32 = WLD-08 of D11), martial
  law (NOB-36 = OVR-07 of A06), baby snatchers (SIE-08 vs CRE-04 of B21), kobolds (SIE-48 vs CRE-12),
  crown's levy (NOB-31 vs MIL-22 of B03), rooms for rent (ECO-51 vs B08).
- **Refactors first:** H1 (step hooks), H2 (one container field per milestone), H3 (missions),
  H5 (strike hook), H7 (ui split) remove most merge conflicts; they are about two sessions together and
  pay for themselves in the first wave.
- **Creatures cannot fight creatures.** Mercenaries, guardian animals (X04) and turned beasts all hit
  this; P04 sidesteps it with hireling dwarves. If more items want allied creatures, add a
  `Creature.side: Side` and a target search over hostile creatures as its own foundation.

**Total estimate:** M7 18 sessions (N01 4, B04 5, N02 4, N03 5), M8 22 sessions (P01 6, P02 4, P03 4,
P04 4, P05 4), foundations and refactors 4 (wave 0 + M7-F), so **about 44 sessions**, or about 6 waves
of 2-6 agents in parallel. Later items roughly 30-40 more (B14 alone 8-12).
