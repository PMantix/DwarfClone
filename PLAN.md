# Implementation plan

How the [WISHLIST](WISHLIST.md) gets built: in waves of parallel work packages, each package done
by one agent on its own branch, merged and measured between waves. The detail for every item
(design, steps with their tests, how the overseer uses it, what to measure, risks) is in the
milestone plans:

| Milestone | Plan | Items | Agent sessions | Waves |
|---|---|---|---|---|
| 1 Forts that live to tell the tale | [plan/m1-survival.md](plan/m1-survival.md) | S02–S10 | ~28 | 0–3 |
| 2 The overseer as a character | [plan/m2-overseer.md](plan/m2-overseer.md) | O01–O12 | ~50 | 0–3 |
| 3 The fort as a society | [plan/m3-society.md](plan/m3-society.md) | C00–C12 | ~57 | 0–3 |
| 4 The world comes to you | [plan/m4-world.md](plan/m4-world.md) | B01–B24 | ~58 | 0–5 |
| 5 War in earnest, 6 Deep fortress | [plan/m5-m6-war-and-depths.md](plan/m5-m6-war-and-depths.md) | A01–B03, F05–D11 | ~104 | 0–5 |
| 7 Dread, 8 A world of forts, Later | [plan/m7-m8-dread-and-world-of-forts.md](plan/m7-m8-dread-and-world-of-forts.md) | N01–P05 | ~44 | 0–5 |

About **340 agent sessions** in all. A session is one agent taking one work package (1–3 of the
WISHLIST's steps) to a tested commit. Run five to eight at a time, that is roughly 60 waves of
elapsed work, so the roadmap is many months of agent time; **Milestone 1 is about 28 sessions in
four waves** and is where to start.

**Other ways to play** (WISHLIST G01–G03): the arena mode is next after Milestone 1's wave 3,
since it is contained and doubles as the combat-tuning tool; the bigger bestiary grows
alongside it; adventurer mode follows Milestones 4 and 8, whose living world and fallen forts it needs.

## How a wave runs

1. **Packages.** Each milestone plan lists its work packages (P0, P1 …) and the wave each belongs
   to. Packages in one wave edit disjoint functions; where two touch the same file the plan names
   the merge order.
2. **One agent per package, each in its own git worktree** (a separate checkout on its own branch,
   `wave/<milestone>-<package>`), started together by a workflow. Each agent:
   - reads its package in the milestone plan and the interface contracts it uses;
   - implements the steps, one commit per step, each with the tests the plan names;
   - leaves `cargo test --release` green and the build free of new warnings;
   - returns its branch, commits, tests added and anything it had to decide.
3. **Review.** A second agent reads each branch's diff against the plan: correctness, the
   overseer actually using the feature, save compatibility (`#[serde(default)]` on new fields),
   hot-path cost, and that the tests prove the step. Findings go back to the package agent once.
4. **Merge.** The coordinator (the main session) merges the branches into master in the plan's
   merge order, resolves conflicts, runs the full test suite, and commits.
5. **Measure.** One 25-fort batch per wave (`--autoplay --runs 25 --years 3 --seed 5000 --out
   batches/<milestone>-w<n>.json`, about 40 minutes), compared with the previous wave's batch.
   A wave that makes forts worse in a way the plan didn't intend is fixed before the next wave.
6. **Report and pause.** After each wave: what merged, the batch comparison, and the tokens the
   wave used, with a running total. The next wave starts when the user says so.

## Rules that keep parallel branches mergeable

These are gathered from all six plans. Wave 0 of each milestone puts them in place before any
parallel work.

- **Systems register in one line.** `Game::step`'s periodic calls move to a schedule
  (`schedule.rs`, M1 P0), job finders to a table (`finders.rs`), overseer extensions to
  `overseer_hooks.rs` (daily, alert, siege-end and brain-sense hooks). A new system adds one
  row, never edits the loop.
- **New state goes in new modules.** A package creates `rescue.rs`, `wildlife.rs`,
  `captives.rs` … rather than growing `game.rs` or `combat.rs`. Where a milestone adds many
  fields to `Game`, they go in one container struct (`Game.dread`, `Game.realm`, M4's state)
  added in that milestone's wave 0.
- **Shared enums are appended once.** Every new `Task`, `Sense`, `Lack`, `Recipe`,
  `ItemKind`, `Mode` variant a milestone needs is added in its wave 0, with all `match` arms.
- **Measure through counters and knobs.** Batch metrics go through `Annals.counters`, tuning
  values through `Game::knob(name, default)` (M1 P0), never new `Trial` fields or constants.
- **Save compatibility.** Every new saved field has `#[serde(default)]`; wave 0 adds a test that
  an old save fixture loads.
- **Pure refactors are checked by result.** A refactor package must leave a fixed-seed batch
  identical to before (world events draw from their own `world_rng` once M4 adds it, so
  world-side changes don't shift fort-side randomness).
- **Big files are split before the waves that crowd them:** `combat.rs` (2,044 lines) into
  siege, wildlife, sneaks and bestiary modules (M1 P3 starts this, M4 P0 finishes it);
  `psyche.rs` (M3 C00); screens out of `ui.rs` into their own files as they're touched.

## Foundations built once

Several plans found the same missing piece. Each is built by the earliest milestone that needs
it, to the fullest contract any plan gives, and later milestones extend it:

| Foundation | Built in | Extended by | Notes |
|---|---|---|---|
| Schedule, finder table, overseer hooks, knobs, counters | M1 P0 | every milestone | M5's step registry and M7's hooks are the same thing |
| Captives (`captives.rs`) | M1 P5 | M4 (`Captive` with news, ransom), M5 (prisoners of war) | one store |
| Grudges → civ standing | M1 P5 (`Annals.grudges`) | M4 F0 `Standing` replaces it | M5 and M8 read `Standing` |
| Event store / saga (`saga.rs`), places, names | M2 W0-B | M3 C00 adds witnesses and structured thoughts; M5's in-tick event queue feeds it | one "who did what to whom, where" log |
| Facing helper (`rel`/`local`) | M2 W0-A | O02, O04 | 55 hard-coded east offsets today |
| Petitions and favour store, `Audience` + `ov_answer` | M3 C00 | M4 F0 (envoys, parleys, tax), M8 | one demand/offer type answered by the overseer's traits |
| World tick (`world_daily`/`world_yearly`), fort as a site, `Party` | M4 F0 | M5 B03, M8 P01–P05 | the world is frozen after embark today |
| Morale, attributes | M5 wave 0 | M3 C08/C12 read morale | |
| Clock: night and moon | first of M6 B13 / M7 FC1 | | no day/night exists today |
| Cases: suspicion, clues, accusations | M3 C07 | M7 FC2 | |
| World file across forts | M8 FC6 | | each save holds its own world today |

## Order

1. **Milestone 1** (waves 0–3), then its 50-fort release batch and v0.3.
2. **Milestone 2**, then **3**: M3's grief and stress changes move every balance number, so
   they come after M1's madness work (S09) has settled.
3. **Milestones 4 to 8** in order. The B06 siege AI (M5) is the riskiest item in the roadmap:
   invaders who dig and batter could push survival back toward zero, so it gets its own batch.

Within a milestone, follow its plan's waves. A later milestone's wave 0 can start while the
previous milestone's last wave is being measured, if it touches different files.

## Decisions for the user

Each plan flags choices it couldn't make alone. The near ones first; each has the plans'
recommendation.

**Milestone 1 (decided October 2026)**
- Bandages may be torn from spare clothing as well as thread and cloth. **Yes.**
- A captured goblin's execution stays off-screen (a thought and a chronicle line). **Yes, mostly:**
  now and then a hard overseer does it in the open, with a card, and the fort reacts.
- A berserk dwarf may kill. **Yes, rarely.**
- The overseer hunts prowling wildlife; legendary beasts only when it is very bold (the Bold
  trait and a strong, armed squad).

**Milestone 2**
- The ghost overlay of the plan on `P` (`p` is stockpiles). *Recommended: yes.*
- Levers linked by a mechanic who walks over (O11) may delay the first gate bridge: keep the
  overseer's first gate link instant? *Recommended: yes.*
- When an overseer dies and is succeeded (O07), are traits the player picked overwritten?
- After embark, should legends hide history the fort hasn't heard of (O09)?

**Milestone 3**
- Keep the current regard list and add trust, respect and fear beside it? *Recommended: yes.*
- Generate gods per civilization when first needed rather than at world generation?
  *Recommended: yes.*
- When the player runs the fort, do petitions and the barony choice pause the game?
- May the overseer execute dwarves in autoplay?
- Keep the fight pit (SOC-26) in M3? Only if cage traps truly hold creatures.

**Milestone 4**
- Can the home civilization fall during play?
- May the player imprison or kill envoys?
- Which starting scenarios ship in v0.6? *Suggested: Frontier, Mining company, Citadel,
  Refugee band, Childhood friends.*

**Milestones 5 and 6**
- Can sappers dig through built walls? *Proposed: no, they batter them.*
- Can the player surrender, as the overseer can?
- Do starving forts eat their dead in long sieges?
- Multi-tile trees and minecarts: move to a later milestone?
- Should the surface have night? (M7 needs a clock anyway.)

**Milestones 7 and 8**
- Money as an abstract treasury or as coin items? *Recommended: abstract.*
- Can a caught vampire be kept (as the bookkeeper)?
- Does the player see secrets the fort doesn't know?
- Cults in scope for v0.9?
- A full statue building, or a temple fixture, for desecration?
- "Continue a world" beside "Continue" on the title screen?
- How small to keep the world economy (P05)?
