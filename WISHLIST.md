# Wishlist

What DwarfClone should do next, and why. Ideas come from playing and watching forts, from
Dwarf Fortress itself (checked against the [DF wiki](https://dwarffortresswiki.org/)) and from
Tarn's [development page](https://www.bay12games.com/dwarves/dev.html), and since October 2026
from a full review of Bay 12's pages: the [idea catalogue](ideas/README.md) compares Dwarf
Fortress with DwarfClone domain by domain and holds 932 ideas, 562 of them placed in the
roadmap below (their IDs, like MIL-03, link to it). We favour fun and
emergent drama over intricate systems: every feature should make a moment you'd tell someone
about. Losing is fun, but a fortress has to live long enough to have a story.

## How to use this

- **Pick work** from the roadmap, top milestone first, items in order. Each item is split into
  steps that ship on their own: one step per commit where possible.
- **Mark progress** in the step list: `[ ]` todo, `[~]` in progress, `[x]` done (add the commit
  hash). When every step is done, move the item to "Done" with a one-line summary.
- **Done when** is the test: each item leaves an automated test behind that proves it. Items
  about balance are measured with the autoplay batch (`--autoplay --runs N --years Y --seed S`).
- **Add ideas** to "Inbox" with a line on why, or pick one from the [idea catalogue](ideas/README.md).
  Promote them into a milestone once they have steps.
- **Steps from the catalogue** carry the idea's ID; its row there has the mechanic, sources and size.
- Sizes: **S** one session, **M** a few sessions, **L** a big system.

## Where we stand (October 2026)

**19 October 2026.** v0.4.0 is released, and Milestone 1 is all but done: 61 of its 62 steps
(only "the shelter sings" is open). Three gaps remain from its goal: no Reckless fort lives three
years, no Warlord fort ever falls, and no fort builds a well or a cistern (the overseer finds no
site for one). Much was built outside the step lists: gods, relics, omens and the wizard; horrors,
the thing in the night and peoples made by villains; arts and culture; named battles, weddings,
lairs and the shelter; the director camera, the mood strip, the highlight reel, life stories and
the news ticker; towers, curtain walls and maturing interiors; gore and close combat, the
bestiary, sound and the arena; the world drawn as it's made, and the title scenes. The steps
these cover are now ticked, and steps they only partly cover say what exists. Steps done:

| Milestone | Done |
|---|---|
| 1 Forts that live to tell the tale | 61 of 62 |
| 2 The overseer as a character | 17 of 86 |
| 3 The fort as a society | 4 of 97 |
| 4 The world comes to you | 4 of 115 |
| 5 War in earnest | 2 of 56 |
| 6 Deep fortress | 2 of 113 |
| 7 Dread | 3 of 36 |
| 8 A world of forts | 0 of 32 |
| Other ways to play | 2 of 9 (the arena, G01, went to Done) |

The rest of this section is the v0.2.0 picture that set Milestone 1's order.

A batch of 25 overseer-run forts (`--autoplay --runs 25 --years 3 --seed 5000`, v0.2.0) gave
**no survivors after three years**, and most fell within the first year. The build before
the planned layouts did the same (0 of the 15 it finished), so this is long-standing, not new.

S01's report on the same batch (saved as `batches/baseline-v0.2.0.json`):

- **Standing:** 13 forts after year 1, 3 after year 2, 1 after year 3 (a Warlord fort).
- **Deaths (451):** bled to death 45%, struck down 35%, fell on an expedition 10%, starvation
  6%, thirst 4%.
- **Killers:** goblins 55% of all deaths; cave horrors, troglodytes, the walking dead, dragons
  and wolves about 5% each.
- **Where:** 58% died during a siege, 36% under the open sky, and **32% inside the shelter**:
  the alert burrow doesn't keep goblins out (the bridge went up in only 22% of first sieges).
- **Sieges:** about five goblins each. The first kills 5.5 dwarves on average and the second
  8.5, while dwarves kill fewer than one goblin per siege.

**The fights up close** (the arena, `cargo test arena -- --ignored --nocapture`; the blow counts
in the batch report; and a tick-by-tick replay, `SEED=5005 STYLE=steady SIEGE=2 cargo test
siege_replay -- --ignored --nocapture`):

- A one-on-one fight is over in 70–100 ticks. Bare-handed, with a pick or with a copper axe a
  dwarf loses to a goblin 76–100% of the time; an iron axe with helm, mail and shield wins
  75–88%, a trained veteran 81–99%. 85–100% of the losers bleed out, within a hundred ticks.
- In real sieges goblins struck civilians 2,130 times (65% drew blood) and soldiers 1,362 times
  (22%: armour works). Soldiers drew blood with 68% of 474 blows but killed fewer than one goblin
  a siege: they swing **wooden crossbows** (1.8 damage, blunt, barely bleeds), and fired only 92
  bolts in 31 sieges.
- When the first siege came, most squads held wooden crossbows and no armour while 10–32 pieces
  of armour lay in stock; three Warlord forts had **no soldiers at all**, lost on expeditions.
- The replay of a typical massacre: the alert sends every civilian into the shelter within 100
  ticks, but **the gate has only one door on a two-tile corridor** (the overseer marks the gate
  done when either door is placed) and no drawbridge (never built for want of mechanisms), so
  all five goblins walk in by tick 200 and kill the cornered civilians one at a time, four or
  five to a dwarf.

So the order is clear: the shelter and the bridge (S04), dwarves who fight back (S04), the
bleeding (S02), and expeditions that cost a tenth of all deaths (a new item below, S06).

What's strong now: the world map and its history, varied sites, planned fortress layouts, the
overseer's ambitions and brain, festivals, heroes, mysteries, legends and the chronicle.

## Roadmap

### Milestone 1: forts that live to tell the tale (v0.3)

Goal: in the 25-fort batch, at least 40% of Steady and Warlord forts and some of every other
style survive three years, and every style still loses forts in memorable ways.

**Result** (release batch, `--autoplay --runs 50 --years 3 --seed 5000`, `batches/m1-release.json`;
the 25-fort before/after is `batches/m1-w3-before.json` / `m1-w3-after.json`): standing after
years 1/2/3 49/39/34 of 50. By style at year 3: Steady 9/10, Warlord 10/10, Builder 9/10, Delver
6/10, Reckless 0/10 (9 after year 1, 4 after year 2; Reckless-only runs on other seeds keep 2-3
of 10). Bled to death 7% of 933 deaths (was 59%), wildlife 0.5 deaths a fort (o01-50: 4.8),
no thirst, expeditions 0.3%, berserk 0.5 a fort; 9 forts fell to sieges, first sieges cost 1.1
dwarves. **Goal mostly met:** Steady and Warlord far past 40%, bleeding well under a quarter,
and forts still fall to sieges, cave horrors, titans, dragons and rocs; but no Reckless fort
lived three years in this batch, Warlord lost none, and no fort built a well (wells and
cisterns never find a site on the new worlds: the overseer's scan finds no water).

**Closing the gaps** (same batch, `batches/m1-close.json`, `--compare batches/m1-release.json
batches/m1-close.json`): standing after years 1/2/3 50/43/41 of 50. By style at year 3:
Steady 9/10, Warlord 10/10, Builder 10/10, Delver 7/10, Reckless 5/10 (on seeds 7000-7029,
Reckless-only, 11 of 30, from 3 of 30). Cisterns filled from a river in 32 of the 40
non-Reckless forts (none before); thirst 1 death, bled to death 10% of 976 deaths. Reckless
forts raise a levy against armies and beasts (levy.rs); Warlords sally against armies shut out
at their bridge (sally.rs, 58 sallies, all won).

**The Warlord's pride** (same batch, `batches/warlord.json`, `--compare batches/m1-close.json
batches/warlord.json`): a sally that breaks an army makes its people swear vengeance, and their
next army comes as a host, larger and past the cap, every one in full kit of their best metal,
fighting on when its general falls; the war-game still sets it against an ordinary army. An
overseer with two sallies won rides out whatever the arena says and won't call the squads back
till a fifth are left. Standing after years 1/2/3 50/44/40 of 50; Warlord 9/10, the other
styles as before. Anvilbeard lost 3 of 5 soldiers breaking a siege in year 3 ("the overseer's
pride has been dearly bought"), and a sworn host of 21 and then a dragon found it without them.
Warlord-only on seeds 7000-7029, 24 of 30 stand (28 of 30 at the branch point, the two lost
there to a cave horror and goblins, never sallying); the new losses follow sallies won dear
against sworn hosts, then whatever comes next. **Still open:** a sally still rarely breaks
outright (4 of 138 sent in the 7000-7029 and release batches); the forts fall a season after
their pride is paid for, not at the gate.

#### S01 Know why forts fall (S)
- **Why:** balance by evidence, not by guess. The batch report gives one line per fort today.
- **Steps:** [x] the batch report tallies causes of death, killers, and siege outcomes (how many
  came, how many died on each side, whether the shelter and the bridge were used) (this commit)
  · [x] a per-fort timeline with `--story`: arrivals, sieges, deaths, ambitions (this commit) ·
  [x] `--out` saves a batch and `--compare A B` prints two side by side (this commit); the
  baseline is `batches/baseline-v0.2.0.json`
- **Done when:** the batch prints the cause table above and a survival count by year.

#### S02 Battlefield medicine (M)
- **Why:** half of all deaths are bleeding. In DF the wounded are carried to bed and sutured.
- **Note:** bleeding adds a third of an edged blow's damage and slows by one a turn, so the
  blood lost grows with the square of the wounds: three cuts kill a dwarf in about a hundred
  ticks, long before anyone could help. The curve needs reshaping, not just faster doctors.
- **Steps:** [x] dwarves who are bleeding stop work and press their wounds (slower bleeding),
  and friends bandage them with cloth or thread where they lie (3813cbf) · [x] the unconscious and the
  crippled are carried to a hospital bed by a doctor or any hauler, even during an alert, once
  the threat is gone (3813cbf) · [x] the overseer always has a doctor and a hospital before the first siege
  (c40e5af) · [x] bleeding is tuned: a cut limb kills in minutes, a gash in hours (b122c30; measured in
  537cc17: 7% of deaths in the release batch, so the default bleed knobs stay)
- **From the Bay 12 review:** [x] Bleeding that leaves time to help ([HLT-03](ideas/body-and-mind.md)) (b122c30) · [x] Wait for rescue ([HLT-07](ideas/body-and-mind.md)) (3813cbf) · [x] Hospital as a stocked place ([HLT-20](ideas/body-and-mind.md)) (c40e5af) · [x] Booze for the wounded ([HLT-21](ideas/body-and-mind.md)) (c40e5af) · [x] Injury readout and medical history ([HLT-47](ideas/body-and-mind.md)) (c40e5af)
- **Done when:** a test where three dwarves are gashed sees them carried in and sutured; in the
  batch, bleeding is no longer more than a quarter of deaths.

#### S03 Sieges that build up (M)
- **Why:** a first siege that wipes out a young fort is no story. DF sends ambushes and small
  raids first, then grows them with the fort's wealth.
- **Note:** sieges arrive on the first day of a season (days 168, 252, 336), the same day as
  migrants, and reach the gate within about 140 ticks.
- **Steps:** [x] scouts see every army coming about a day ahead (named leader, size; a card),
  and the overseer shelters everyone and raises the bridge before it arrives; the first is a
  raiding party of two or three, later ones grow with population, wealth and every siege broken
  (this commit) · [x] besiegers shut out make off with valuables lying outside (this commit) ·
  [x] the leader is named in the warning and killing them routs the rest (already so) · [x]
  they remember: a broken siege brings a bigger one, and they say so (this commit) · [x]
  captives (1e1315c) · [x] now and then the scouts miss a later army, which is at the edge of
  the map before anyone knows (`siege.unseen`, 537cc17)
- **Result** (batches/s03.json against s04d): forts standing after years 1/2/3 went from
  17/7/7 to 22/15/12; the first siege kills 2.2 dwarves on average (was 9.2) and took one
  fort (was 12); later sieges of ~8 still took five. Steady and Warlord: 3 of 5 each standing
  after three years.
- **Done when:** the batch shows first sieges killing fewer than three dwarves on average and
  later sieges still killing whole forts sometimes.

#### S04 An overseer that defends (M)
- **Why:** the overseer has a drawbridge, cage traps and a shelter burrow, but forts still die in
  a day. It should defend like a nervous player.
- **Steps:** [x] a whole gate: both doors of the two-wide gatehouse and both halves of the
  drawbridge and its lever, checked and replaced if missing (this commit) · [x] mechanisms early:
  the mechanic's shop is the third workshop (this commit) · [x] squads fight with crossbows
  until there are melee weapons for all, and anyone holding a crossbow keeps a quiver of bolts
  (this commit) · [x] the only squad never goes on an expedition: another must hold the gate
  (this commit) · [x] the shelter covers every hall, room and stair the overseer digs (it used
  to skip rooms not yet dug, and the halls of planned layouts, so sheltering dwarves died of
  thirst beside full barrels) (this commit) · [x] fleeing dwarves run somewhere well away,
  deeper into the fort, rather than into a corner (this commit) · [x] the bridge goes up
  sooner: 150 ticks after an army appears, and whoever stands by the lever (the squad's
  station) throws it at once; traps wait for the bridge's mechanisms; the exploring shaft is
  dug inside the gate, not beside the stair (goblins went straight down it) (this commit) ·
  [x] soldiers hold the gatehouse (stationed by the lever) · [x] after a siege
  it learns: more traps where the enemy came in, a second squad, walls (4be72f5) · [x] cavern breaches
  get a door the moment they're dug into (beasts can't open doors) (this commit)
- **Latest** (batches/s04d.json): 17/7/7 standing; the bridge is up in 46% of first sieges,
  58–70% of later ones. What's left is the first siege: it comes on day 168, mostly before the
  bridge is built, and catches dwarves in the open (44% of the dead died under the sky). That
  is S03's to fix (a smaller first raid, seen coming).
- **Before that** (batches/s04c.json against batches/before-S04.json): forts standing after years
  1/2/3 went from 15/1/0 to 18/9/7; later sieges mostly fail now (the bridge is up in 82–86% of
  them), but the first siege still kills 9 dwarves on average and takes 13 forts of 25, because
  the bridge is up for only a third of them. Thirst deaths rose (22 → 58) with forts living
  longer: S05.
- **From the Bay 12 review:** [x] The overseer learns from a siege ([SIE-01](ideas/war.md)) (4be72f5)
- **Done when:** a scripted siege against a fort with shelter, bridge and squad kills fewer than
  half the goblins' worth of dwarves; the batch meets the milestone goal.

#### S06 Expeditions worth the risk (S)
- **Why:** 43 of 451 deaths (10%) were dwarves who fell on expeditions, most of them in the
  first year, when a fort can least spare them.
- **Steps:** [x] the overseer sends expeditions only with a strong, armed squad and spare
  dwarves at home (e54ad30) · [x] odds are shown before sending, and a squad that's losing comes home early
  with its wounded (e54ad30)
- **From the Bay 12 review:** [x] Companions turn back ([OUT-17](ideas/world-and-overseer.md)) (e54ad30) · [x] Field rations and care packages ([OUT-19](ideas/world-and-overseer.md)) (e54ad30) · [x] The tactician matters ([OUT-20](ideas/world-and-overseer.md)) (e54ad30)
- **Done when:** expedition deaths fall below 5% of all deaths in the batch.

#### S05 Food and water a fort can rely on (S)
- **Why:** dwarves died of thirst and hunger in peacetime, often beside full stores.
- **What the traces found:** most were mad (melancholy after a massacre's grief; a mad dwarf
  only wanders); sane ones were trapped by water: a stair dug through a heavy aquifer, or a hall
  opened into a lake's bed, flooded the fort's main level and cut the halls off from the food.
- **Steps:** [x] melancholy can lift, given time and someone who cares still alive, and a
  melancholic dwarf still sips and picks at food when it's that bad (this commit) · [x] desperate
  thirst interrupts a meal or a nap (this commit) · [x] the stair is dug where water can't pour
  in, under rock, and stops above a heavy aquifer; no hall is cut beside or under water; no
  level below a heavy aquifer (this commit) · [x] the overseer reads the announcements: what's
  short (stone, wood, blocks, mechanisms, drink, food, a workshop, a manager…) is remembered
  as a trouble, and it sees to it (miners to quarry, woodcutters out, orders raised, hands to
  farming), sooner or later by its style and with a manager's help (this commit) · [x] only a
  delver digs levels down toward the caverns (a spider came up one fort's stair) (this commit)
  · [x] wells and cisterns inside the fort (0b0db40; on the new worlds, cisterns fed down a
  shaft from a river above the halls, ec910b9: 32 of 40 non-Reckless forts in m1-close)
· [x] a dwarf in a fey mood
  gets the workshop and materials they demand: the overseer treats those as troubles of its own,
  and urgent ones (this commit)
- **So far:** thirst and starvation deaths 93 (s03) → 66 (s05b) → 51 with the troubles
  (batches/troubles.json); no sane or melancholic dwarf died of thirst there, all 38 thirst
  deaths were dwarves driven mad by a failed fey mood, which this commit goes after.

#### S07 Dwarves who fight back (M)
- **Why:** Soldiers swing wooden crossbows, leave armour in stock and fired 92 bolts in 31 sieges; the overseer never sends anyone after the wolves that now cause a fifth of deaths.
- **Steps:** [x] Metal crossbows and bolts ([MIL-01](ideas/war.md)) (141b487) · [x] Kit on before the first siege ([MIL-02](ideas/war.md)) (141b487) · [x] Archers who act like archers ([MIL-05](ideas/war.md)) (4fe9d99) · [x] The overseer hunts predators ([MIL-03](ideas/war.md)) (2423aec) · [x] Wounds that leave time to help ([MIL-04](ideas/war.md)) (b122c30, openings 4fe9d99)
- **Done when:** A scripted siege sees soldiers in metal and armour killing at least one goblin each; in the batch, bleeding is under a quarter of deaths.

#### S08 Wild things with sense (L)
- **Why:** Wolves and other wildlife charge whole squads like berserkers; real predators pick off stragglers and break off when hurt.
- **Steps:** [x] Predator discretion ([CRE-01](ideas/myth-and-depths.md)) (e54f3d8) · [x] Fire and intimidation ([CRE-02](ideas/myth-and-depths.md)) · [x] Wary once wounded ([CRE-03](ideas/myth-and-depths.md)) (e54f3d8) · [x] Stalking predators ([CRE-15](ideas/myth-and-depths.md)) (e54f3d8) · [x] Silent woods ([CRE-22](ideas/myth-and-depths.md)) · [x] Wildlife from lairs ([CRE-23](ideas/myth-and-depths.md)) · [x] Living populations ([CRE-29](ideas/myth-and-depths.md))
- **Done when:** Deaths to wildlife halve in the batch, and a wolf pack shadows a lone woodcutter rather than a squad.

#### S09 Madness that mends (S)
- **Why:** Every thirst death in the batches was a mad dwarf; breakdowns should be dramatic, varied and sometimes recoverable rather than a slow walk to death.
- **Steps:** [x] Berserk and shifting breakdowns ([MND-06](ideas/body-and-mind.md)) (116a9ae) · [x] Catatonia ([MND-31](ideas/body-and-mind.md)) (116a9ae)
- **Done when:** No mad dwarf dies of thirst beside full barrels in the batch; a berserk dwarf is subdued or recovers.

#### S10 Knobs for the batch (S)
- **Why:** Balance by evidence: the numbers the batches tune should live in one data file, not scattered constants.
- **Steps:** [x] Tunable thresholds ([OTH-04](ideas/world-and-overseer.md)) (ceddd9b; siege, megabeast and thief triggers 537cc17) · [x] Path throttling ([OTH-06](ideas/world-and-overseer.md)) (537cc17)
- **Done when:** A batch can be rerun with a changed threshold from the command line.
- **Result:** the profile (`TRIAL_PROF=1`, now with time per job finder) showed path searches
  cheap (~150 nodes a tick) and the dwarves' finders, hauling and farming above all, most of
  a tick: idle and strolling dwarves asked every finder every 30 ticks, each scanning thousands
  of items. Resting a finder that found nothing cut the batch from 177 to 117 thread-seconds a
  fort-year; a besieger shut out for 1000 ticks searches 14 times instead of 67.

#### S11 The shelter holds (M)
- **Why:** about 30% of the dead died inside the shelter, and long sieges could end in thirst
  with the drink in a pile outside it.
- **What the traces found** (`shelter::tests::shelter_breach_trace`, seeds 5001, 5003, 5006,
  5007): two causes. The gate's own tiles (the doors in front of the bridge) were part of the
  shelter, so civilians running in stopped on the doors, the bridge went up behind them, and
  the goblins at the foot of the stair cut them down there (14 of the 33 deaths in the baseline
  10-fort run). And halls or tunnels that came up under the sky let besiegers walk round the
  raised bridge into the shelter (one Delver fort lost 8 in one siege that way).
- **Steps:** [x] the gate (stair foot, doors, bridge) is no part of the shelter, so those
  running in go on past the bridge · [x] a larder inside the shelter, kept in
  drink and meals by the haulers · [x] every other way in (to the sky, or into the stair from
  the surface) is walled up, daily and the moment the alert goes up · [x] the shelter sings
  (a while into an alert those sheltering sing together and take heart; a lament once the
  siege has cost a life) (this commit)
  · [x] a siege sheltered through without loss is told
- **Done when:** a long siege doesn't end in thirst while there's drink; a way in past the gate
  is walled in an alert; deaths inside the shelter fall in the 10-fort batch.
- **Result** (`--autoplay --runs 10 --years 2 --seed 5000`, `batches/m1-shelter.json`): deaths
  inside the shelter 33 of 115 before, 2 of 76 after (both to a cave horror, outside any siege);
  thirst deaths 0 before and after (one appeared while walls could still split the shelter, and
  went when they no longer could). 26 sieges were sheltered through without loss.

### Milestone 2: the overseer as a character (v0.4)

Goal: watching a fort, you can tell which overseer built it, and its halls look planned and lived in.

#### O01 Overseer personalities (S)
- **Why:** the layouts made each fort look different; the overseer itself should feel like
  someone. Today a style picks ambitions and a layout.
- **Steps:** [x] each overseer gets a name and two traits (cautious, bold, vain, pious, greedy,
  sentimental, stubborn, cheerful) that lean its ambitions (a new sense in its brain, "Its
  temper"), sway its layout, sharpen or blunt its eye for what the fortress lacks, and change
  how soon it bars the gate and how many it keeps under arms (7579248) · [x] the brain and
  the autoplay screen show who it is (7579248) · [x] `A` then `r` rolls another of the same
  style (7579248) · [x] its notes on what it saw to speak in its voice ("Uneasy, it saw the
  fortress short of stone…") (7579248) · [x] pick the traits by hand (6892e14: `t` on the
  Autoplay screen, `--traits a,b` for batches) · [x] more of its
  notes in its voice (e79d14e: six openings a trait; its halls, feasts and eulogies too; 6892e14:
  notes tagged by kind with openings per trait and kind, and it mourns, sneers or frets over deaths
  and sieges by its traits)
- **From the Bay 12 review:** [x] Watch and nudge ([OVR-24](ideas/world-and-overseer.md); 6892e14: its
  next step in the top bar and brain, and danger cards offer b raise the bridge · s shelter · m muster
  · d stand down, which it obeys for half a day; a stubborn one sometimes won't be told)
- **Done when:** two overseers with the same style but different traits build visibly different
  forts from the same seed.

#### O02 Plans that fit the land (M)
- **Why:** every plan runs east from the gate; on a narrow site most of it is wasted.
- **Steps:** [x] a plan can face east, west, north or south, and the overseer picks the side with
  the most solid rock (3bd2848; the plan turns, the gatehouse itself still runs east) · [x] a level's plan stretches to the space available, and deeper levels
  get their own layout (workshops on one, bedrooms on another, like DF forts) (3bd2848: levels
  for bedrooms, workshops, stores and great halls, each with its own layout and rooms) · [x] `P` (`p` is stockpiles) shows the
  plan as a ghost overlay on the map: halls and rooms still to come (3bd2848)
- **Done when:** on a site with rock only to the west, a planned fort still fills its plan.

#### O03 Halls that look lived in (M)
- **Why:** finished forts in DF are smoothed, engraved, furnished and named.
- **Steps:** [x] the main halls are smoothed, then engraved, as the fort grows rich (efa01ab; halls.rs, this commit: the plan's own halls, stretch by stretch out from the stair, from the time the fort is settled; smoothed walls drawn as polished double lines, engraved ones gilt) · [x] bedrooms
  get a cabinet and a chest; nobles get suites with an office and a dining room (feat/interiors:
  architect.rs; also bedrooms of their own, the dormitory retired, gathering halls, the industry
  quarter and rebuilding in maturity.rs) · [x] the atrium
  and the hub get statues and the monument (a rotunda holds the monument, gathering
  halls and temples get statues, efa01ab; pillars round the monument, ad88122; this commit: the
  plan keeps the atrium's and hub's hearts for the monument with statues round them, and statues
  down a great hall) · [x] halls are named after heroes and events
  ("the Hall of the Unbeaten") and the names show in the look panel and the chronicle (rooms
  are named in the look panel, a tavern gets a name of its own, efa01ab; a place takes a name
  from a deed done there, 6567979; naming.rs, this commit: a great hall is named for the fort's
  hero or noble, and renamed for a leader or beast slain in it, three dead in a day, a stand or
  a wedding; the gate for its first held siege; announced, noted, in legends and chronicle)
- **From the Bay 12 review:** [x] Trophies of war ([CRF-01](ideas/economy.md), this commit: the skulls of dragons, named beasts, horrors and siege generals, set up on displays in the great hall and admired; their people come back for them) · [x] Named weapons ([CRF-02](ideas/economy.md), this commit: a weapon is named at its fifth kill, with a legend, and its great kills go into the legends) · [x] Inscriptions and maker's marks ([CRF-06](ideas/economy.md), this commit: superior work bears its maker's mark, exceptional work an inscription) · [ ] Name creator ([CRF-08](ideas/economy.md); partly: battles, places, taverns, instruments and weapons get names) · [x] Jeweler and decorations ([CRF-13](ideas/economy.md), this commit: the craftsdwarf sets cut gems into furniture and figurines; a rich overseer orders it) · [x] Gems in engravings, engraved built walls ([CRF-12](ideas/economy.md), this commit) · [x] Statues and figurines of things ([CRF-21](ideas/economy.md); statues and crafts show the fort's legends and heroes, f6ad60c; this commit: and the beasts it has met, its animals, the generals at its gate, its drawbridge, well and monument) · [x] Places named for what happened there ([WLD-02](ideas/world-and-overseer.md), 6567979)
- **Done when:** a three-year fort has smoothed main halls, furnished bedrooms and named halls.

#### O04 Defences in the plan (M)
- **Why:** DF forts are famous for moats, trap halls and walled courtyards.
- **Steps:** [ ] the gatehouse grows with the fort: a trap corridor, then a moat under a
  drawbridge, then towers with fortifications (towers: ad88122, on the surface: a curtain
  wall with gate towers, battlements, a walkway, an arch and a drawbridge; watchtowers whose
  watch sees armies sooner; marksdwarves shoot down from the towers; the trap hall behind the
  gate's cages, named like a mouth, "the Hall of Teeth", this commit; made to kill, a70e977 and 25411f1: a let-in overseer clears the near row of cages and shuts the army in from a second lever behind the hall, trap-wounded bleed-outs count in its toll, about a third of the goblins that come in die of it; a pit bridge past the hall, this commit: a two-wide drawbridge on its own lever with a pit two levels deep cut under it, raised under the enemy as they crowd onto it, the fall breaking bones, "the Belly" filled from the well as a pond that drowns them) · [x] Warlord and cautious
  overseers build them sooner (the walls and towers, ad88122; the trap hall at once, and a bold
  or warlike overseer lets the army into it before raising the bridge, this commit) · [ ] a siege that breaks on
  the defences goes into the legends (partly: a siege with
  blood spilt is named and goes into the legends, 15be5b2; one that breaks bloodless on the walls doesn't)
- **From the Bay 12 review:** [x] Pressure plates ([ENG-06](ideas/economy.md), this commit: a foe's weight fires the spikes they're linked to) · [x] Upright spikes ([ENG-04](ideas/economy.md), this commit: fired by a plate or a lever) · [x] Pits and ponds ([ENG-03](ideas/economy.md), this commit: a pit under a drawbridge, filled bucket by bucket from the well; what falls in and can't swim drowns) · [x] Traps jammed with bodies ([ENG-01](ideas/economy.md), this commit: two bodies or cages on a trap jam it; the overseer has them dragged out after a siege) · [ ] Trapdoors and chutes ([ENG-07](ideas/economy.md); partly, this commit: channel under any drawbridge and raising it drops whoever stands on it into the pit like a trapdoor; no hatches or chutes) · [ ] Nets and tripwires ([ENG-08](ideas/economy.md); partly: tripwires of thread throw a foe down under the traps behind, this commit, headlong onto the spikes, which the wire springs, a70e977; no nets) · [x] Better traps ([ENG-19](ideas/economy.md), this commit: stone-fall traps drop their stone three times; weapon traps already strike three) · [x] Moats and wide drawbridges ([ENG-20](ideas/economy.md), this commit: the pit bridge, two wide on one lever, over a dug pit that becomes a pond) · [ ] Deliberate flooding ([ENG-02](ideas/economy.md))
- **Done when:** a Warlord fort has a moat and a trap corridor before its third siege.

#### O05 Spectator mode (M)
- **Why:** An autoplay fort is a show: the player should see moods at a glance, have the camera cut to the drama, and replay the best moments.
- **Steps:** [x] Mood strip ([UI-01](ideas/world-and-overseer.md), 4c47723: faces in the top bar, click to go, the overseer reads them) · [x] Director camera ([UI-03](ideas/world-and-overseer.md), 2edf40e: tours the fort when idle, fights first; music and dances ffc0b89) · [x] Sound-based reports ([UI-06](ideas/world-and-overseer.md), e4dcc98 and this commit: fights, deaths, cave-ins and fires out of sight come in as sounds with a direction or a place; Enter goes to look) · [x] Last words ([UI-02](ideas/world-and-overseer.md), e4dcc98: of a spouse, a child, a grudge, the killer, a named weapon, their craft; heroes boast and the mad ramble; quoted in the overseer's eulogies and the chronicle) · [x] Announcement control ([UI-07](ideas/world-and-overseer.md), this commit: `a` on the reports screen; each kind pauses, moves the camera or hides, kept in saves/announce.json) · [x] Step size ([UI-32](ideas/world-and-overseer.md), this commit: `s` a tick, `T` an hour, `D` a day) · [x] Highlight reel and replay ([UI-04](ideas/world-and-overseer.md), db1d6b3: clips of every big moment, death and wedding, named battles, `h` plays them, the overseer's moment of the year; f824da4: filmed as the map draws it while a screen is up, sagas' turns and endings filmed, `p` plays the best moments one after another on the map) · [x] Export the fort ([UI-10](ideas/world-and-overseer.md), f824da4: `e` on the Highlights screen writes `exports/<fort>-<year>/`: a world-map-style PNG and a text map of every level, the best moment as a GIF, the story as `fort.html` and `story.txt`) · [x] Ambient particles: rain, snow, leaves, fireflies, dust (fa0b163) · [x] A news ticker when the announcements are hidden or full (e2e3f3f)
- **Done when:** A three-year autoplay fort produces a highlight reel that replays its sieges and deaths.

#### O06 Know your dwarves (M)
- **Why:** Every dwarf should be a story you can read: their life, their family, how they died.
- **Steps:** [x] A dwarf's life story ([UI-05](ideas/world-and-overseer.md), f031dd9: the Life page with a portrait, the overseer's eulogies) · [x] Unit list with stress ([UI-13](ideas/world-and-overseer.md), 44de295: citizens, pets and livestock, visitors, the dead and missing; sorted by stress, happiness, job or name) · [x] Live unit sheet ([UI-14](ideas/world-and-overseer.md), ef60d70, 44de295: tabs for now, thoughts, life and health, a side pane beside the live map, `f` follows) · [x] Family tree screen ([UI-15](ideas/world-and-overseer.md), this commit: `F` on the Life tab or page; grandparents to grandchildren, past spouses, siblings, animals, friends and grudges marked ♥ and ✗, the dead with † and where they lie or that they lie unburied; Enter opens a sheet, `t` their tree; families are buried side by side) · [x] Death scenes in the chronicle ([UI-08](ideas/world-and-overseer.md), f031dd9: a line for each notable dwarf lost that year; told as a scene, where, when, in which siege, the last words and who was left behind, this commit) · [x] Status readouts ([UI-35](ideas/world-and-overseer.md), this commit: a Status tab on the sheet: hunger, thirst and rest, bleeding and how long they have, bed, the next treatment and who gives it, wounds coloured by how bad, their animals)
- **Done when:** Any dwarf's sheet shows a life story drawn from the chronicle and a family tree.

#### O07 A mortal overseer (M)
- **Why:** The overseer should be a dwarf of the fort: it can be doubted, deposed, killed and succeeded, and its traits drift with what it lives through.
- **Steps:** [x] A mortal overseer and succession ([OVR-02](ideas/world-and-overseer.md), this commit: the overseer is a dwarf of the fort; killed, mad, jailed, deposed or out-voted, it is succeeded with a card, a legend and a chronicle line) · [x] Faith in the overseer ([OVR-18](ideas/world-and-overseer.md), this commit: faith moves with deaths, hunger, routs and feasts; low faith brings grumbling, defiance, food riots, the Doubt sense, a reign saga and deposition) · [x] Traits that drift ([OVR-10](ideas/world-and-overseer.md), e337f57: massacres, routs, famine, artifacts, the monument and grief change its traits, once a season at most) · [x] Spin ([OVR-05](ideas/world-and-overseer.md), e337f57: a vain or doubted overseer tells a lost siege as a glorious stand; those who were there may find the lie out) · [x] The overseer's journal ([OVR-06](ideas/world-and-overseer.md), e337f57 and this commit: a page per year of each reign in its own hand, `J` or `j` on the Autoplay screen)
- **Done when:** An overseer killed in a siege is succeeded by another with different traits, and the chronicle marks the change of reign.

#### O08 Orders that think (L)
- **Why:** The overseer's industry spams announcements for missing inputs instead of planning the chain that makes them.
- **Steps:** [x] Orders queue their own inputs ([ECO-11](ideas/economy.md), 4b7049a) · [x] Conditional work orders ([ECO-04](ideas/economy.md), 4b7049a) · [x] Material-specific orders ([ECO-34](ideas/economy.md), 4b7049a) · [x] Mandates feed orders ([ECO-35](ideas/economy.md); when the overseer runs the fort, 4b7049a) · [x] Workshop masters and apprentices ([ECO-13](ideas/economy.md); the master works a grade better, apprentices beside them learn half as fast again; the overseer names masters weekly, this commit) · [x] Job priorities and postings ([ECO-20](ideas/economy.md); 1-7 in designate mode, `p` on work orders, urgent work pulls dwarves off minor jobs, this commit) · [x] Vein mining ([ECO-14](ideas/economy.md); done in O10, fc6dabe) · [x] Records of production ([ECO-15](ideas/economy.md); the ledger, `l` on the orders screen, chronicle and game-over lines, this commit) · [ ] Workshop task tooltips ([ECO-42](ideas/economy.md))
- **Done when:** An order for steel queues the coke, ore and flux it needs, and no announcement repeats a missing input.

#### O09 Legends worth telling (M)
- **Why:** The fort's history should read like a saga in the overseer's voice: named battles, named places, remembered battlefields.
- **Steps:** [x] Named episodes and battles ([WLD-01](ideas/world-and-overseer.md), 15be5b2) · [x] Old battlefields remembered ([WLD-12](ideas/world-and-overseer.md); the place is named and later fights there are named after it, 6567979; bones left lying, passers-by remember, anniversary vigils and feasts, ghosts walk the field, the overseer's memorial slab, this commit) · [x] Ages named by the world ([WLD-13](ideas/world-and-overseer.md); this commit: history read into named ages, a new age called at the turn of a year when the world changes, a legend and a chronicle line, the legends header names it) · [x] Sagas: named story arcs told on illuminated cards (f9d85c0, bde720d, this commit) · [x] Legends that drift ([WLD-09](ideas/world-and-overseer.md); this commit: old tales grow, take a hero's name, turn to "They say..." or fade unless carved; each people tells them its own way (`tales::tale_for` for A7); bards make tales up) · [x] Forgotten lore ([WLD-18](ideas/world-and-overseer.md); this commit: at embark all but our people, the country round and living memory is forgotten, gaps in the legends; ruin expeditions, the sealed hall and bards bring it back; the overseer values ruins more when most is lost)
- **Done when:** A siege goes into the chronicle under a name, and the place it broke is named after it.

#### O10 A miner's craft (M)
- **Why:** The Delver style needs a craft to be good at: following veins, prospecting, surviving cave-ins.
- **Steps:** [x] Dig speed by pick and rock ([DIG-06](ideas/myth-and-depths.md); fc6dabe) · [x] Follow the vein ([DIG-11](ideas/myth-and-depths.md); fc6dabe) · [x] Prospecting and finds ([DIG-12](ideas/myth-and-depths.md); fc6dabe) · [x] Warm and damp stone ([DIG-21](ideas/myth-and-depths.md); this commit: a miner stops short of hidden water or magma and says so; `w` on the work orders screen digs on anyway; the overseer leaves it be unless it is bold, reckless or a delver after magma; breaking into hidden water is a Flood! card) · [x] Surviving cave-ins ([DIG-07](ideas/myth-and-depths.md); this commit: rock falling on a dwarf buries them alive more often than not, a Trapped! card, the miners dig the rubble first and the overseer sends more picks; dug out within a day they live, a Dug out alive! card, else they suffocate; a creaking ceiling now and then gives way) · [x] Gems in their host rock ([DIG-33](ideas/myth-and-depths.md); 8c721e8: garnet in basalt, onyx in limestone, jasper in sandstone, lapis in marble, amethyst in granite) · [x] Digging out boulders ([DIG-34](ideas/myth-and-depths.md); fc6dabe)
- **Done when:** A Delver overseer follows a vein and finds more ore than a Builder from the same seed.

#### O11 A builder's hands (M)
- **Why:** The rest of DF's construction kit: roads, overlapping zones, levers installed by a mechanic.
- **Steps:** [x] One-column stairs ([ENG-09](ideas/economy.md), 9948eda) · [x] Builders don't wall themselves in ([ENG-10](ideas/economy.md), 9948eda) · [x] Bulk rooms and overlapping zones ([ENG-17](ideas/economy.md), 9948eda) · [x] Named halls ([ENG-16](ideas/economy.md); the overseer names its rooms, efa01ab; the player with `N` in look or query, 9948eda; not yet on map labels) · [x] Build over built floors ([ENG-45](ideas/economy.md), 9948eda) · [ ] Better levers ([ENG-33](ideas/economy.md)) · [ ] Roads and wagon access ([ENG-35](ideas/economy.md))
- **Done when:** A dining hall that is also the meeting area works for both, and linking a lever sends a mechanic.

#### O12 Inspired engineers (M)
- **Why:** Great works should come from named dwarves with ideas, giving the overseer ambitions it didn't plan.
- **Steps:** [ ] Blueprints ([CRF-03](ideas/economy.md)) · [ ] Automatons ([CRF-28](ideas/economy.md)) · [ ] Luxury trinkets ([CRF-35](ideas/economy.md))
- **Done when:** A dwarf's inspiration becomes an overseer ambition and a finished work bearing their name.

### Milestone 3: the fort as a society (v0.5)

Goal: the fort runs on its own politics and feelings. Dwarves remember, grieve, feud, marry, drink, pray and lobby, and the overseer has to live with them.

#### C01 The inner life (L)
- **Why:** Dwarves should remember, relive, harden and change, and the fort's mood should be readable at a glance.
- **Steps:** [ ] Richer thoughts ([MND-23](ideas/body-and-mind.md)) · [ ] Many emotions ([MND-22](ideas/body-and-mind.md)) · [ ] Two-speed feelings ([MND-01](ideas/body-and-mind.md)) · [ ] Fort mood at a glance ([MND-20](ideas/body-and-mind.md); partly: the mood strip, 4c47723; no list by stress) · [ ] Memories that come back ([MND-02](ideas/body-and-mind.md); partly: the Life page tells the strongest memory of each season, f031dd9; none are relived) · [ ] Hardened by tragedy ([MND-04](ideas/body-and-mind.md); partly: each sight of carnage hardens a dwarf a little, d28911c) · [ ] Stress tuning ([MND-59](ideas/body-and-mind.md)) · [ ] Personality drift ([MND-03](ideas/body-and-mind.md))
- **Done when:** A dwarf who lived through a massacre relives it a season later and comes out changed, and the change shows on their sheet.

#### C02 Grief and vengeance (L)
- **Why:** A death should ripple through the fort: mourners who wander, vows against the killer, panic at the sight of the place it happened.
- **Steps:** [ ] Grief needs a body ([MND-21](ideas/body-and-mind.md)) · [ ] Grief wandering ([MND-11](ideas/body-and-mind.md)) · [ ] Last words ([MND-12](ideas/body-and-mind.md)) · [ ] Vengeance vow ([MND-05](ideas/body-and-mind.md)) · [ ] Traumatic triggers ([MND-07](ideas/body-and-mind.md)) · [ ] Buried resentment ([MND-17](ideas/body-and-mind.md)) · [ ] Coming home ([MND-37](ideas/body-and-mind.md))
- **Done when:** A dwarf whose spouse was killed swears vengeance and seeks out that enemy in the next siege.

#### C03 Feuds and factions (L)
- **Why:** Duels and grudges (F10) should grow into fort politics: sides taken, brawls that spread, dwarves shunned.
- **Steps:** [ ] Grudges from pets ([SOC-07](ideas/society.md)) · [ ] Slow road to violence ([SOC-08](ideas/society.md)) · [ ] Shunning ([SOC-03](ideas/society.md)) · [ ] Factions in a brawl ([SOC-04](ideas/society.md)) · [ ] Nicknames for anyone ([SOC-10](ideas/society.md)) · [ ] Outsider suspicion ([SOC-15](ideas/society.md)) · [ ] Relationship dials ([SOC-38](ideas/society.md)) · [ ] Ostracism ([MND-16](ideas/body-and-mind.md))
- **Done when:** A brawl between two dwarves draws in their friends on either side.

#### C04 Fort society in motion (L)
- **Why:** Dwarves goad, envy, argue and cheer together; small social weather between the big events.
- **Steps:** [x] Victory cheer ([MND-09](ideas/body-and-mind.md), c1a411a) · [ ] Goading and dares ([MND-13](ideas/body-and-mind.md)) · [ ] Envy of rooms, gear and titles ([MND-19](ideas/body-and-mind.md)) · [ ] Impressions and compatibility ([MND-26](ideas/body-and-mind.md)) · [ ] Long-lived ties and social clumping ([MND-27](ideas/body-and-mind.md)) · [ ] Arguments ([MND-28](ideas/body-and-mind.md)) · [ ] Crowd moods ([MND-52](ideas/body-and-mind.md)) · [ ] Rebellious youth ([MND-18](ideas/body-and-mind.md)) · [ ] Altruism ([MND-53](ideas/body-and-mind.md))
- **Done when:** A fort's gossip shows envy and friendship forming without any death or siege.

#### C05 Family sagas (L)
- **Why:** Births, weddings and deaths should be stories the chronicle can tell.
- **Steps:** [x] Named after heroes ([SOC-01](ideas/society.md), bda7b14) · [x] Weddings as events ([SOC-02](ideas/society.md), 54a152c) · [ ] Orphans adopted ([SOC-13](ideas/society.md)) · [ ] Siblings, grandparents, heirs ([SOC-24](ideas/society.md)) · [ ] Mothers and babies ([SOC-25](ideas/society.md)) · [ ] Sibling rivalry and favourites ([SOC-23](ideas/society.md)) · [ ] Inheritance ([SOC-51](ideas/society.md)) · [ ] Heredity ([SOC-52](ideas/society.md)) · [ ] Coming-of-age rite ([SOC-12](ideas/society.md)) · [ ] Rebellious youth ([SOC-22](ideas/society.md))
- **Done when:** A child named after a fallen hero appears in the chronicle with the reason.

#### C06 The council (L)
- **Why:** Nobles issue mandates that nothing reads; they should be a pressure group the overseer has to answer, by its traits.
- **Steps:** [ ] Overseer answers mandates ([NOB-01](ideas/society.md)) · [ ] Mandate flavour ([NOB-39](ideas/society.md)) · [ ] Nobles play favourites ([NOB-07](ideas/society.md)) · [ ] Office envy ([NOB-40](ideas/society.md)) · [ ] Export bans ([NOB-43](ideas/society.md)) · [ ] Choose your baron, or refuse ([NOB-11](ideas/society.md)) · [ ] The council lobbies the overseer ([NOB-02](ideas/society.md)) · [ ] Spin ([NOB-09](ideas/society.md)) · [ ] Elections and complaints ([NOB-45](ideas/society.md)) · [ ] A council's counsel ([OVR-22](ideas/world-and-overseer.md)) · [ ] The council lobbies ([OVR-03](ideas/world-and-overseer.md)) · [ ] Noble displeasure ([OVR-29](ideas/world-and-overseer.md)) · [ ] Petitioners at the leader's door ([OVR-30](ideas/world-and-overseer.md))
- **Done when:** A mandate becomes a trouble the overseer meets or refuses, and refusing costs it the noble's favour.

#### C07 Crime and punishment, for real (L)
- **Why:** Crimes should have witnesses, doubt and consequences: theft by citizens, wrong accusations, banishment, prison unrest.
- **Steps:** [ ] Witnesses report crimes ([NOB-05](ideas/society.md)) · [ ] Graded punishments ([NOB-04](ideas/society.md)) · [ ] Banishment and expulsion ([NOB-03](ideas/society.md)) · [ ] Missing, then dead ([NOB-06](ideas/society.md)) · [ ] Theft as a citizen crime ([NOB-20](ideas/society.md)) · [ ] Unsolved crimes and accusations ([NOB-19](ideas/society.md)) · [ ] Bribery ([NOB-10](ideas/society.md)) · [ ] Prison life ([NOB-52](ideas/society.md)) · [ ] Prison unrest ([NOB-14](ideas/society.md)) · [ ] Hammerer and executions ([NOB-38](ideas/society.md)) · [ ] Public punishment ([NOB-37](ideas/society.md)) · [ ] Witch hunts and scapegoats ([NOB-22](ideas/society.md))
- **Done when:** A theft with one witness leads to an accusation that is sometimes wrong.

#### C08 The bottle (M)
- **Why:** Drink is the fort's fuel; dwarves should get drunk, get dry, and get into trouble either way.
- **Steps:** [ ] Drunkenness as a level ([HLT-04](ideas/body-and-mind.md)) · [ ] Drink withdrawal ([HLT-09](ideas/body-and-mind.md)) · [ ] Drinking yourself to death ([HLT-10](ideas/body-and-mind.md)) · [ ] Tripping ([HLT-24](ideas/body-and-mind.md)) · [ ] Contact poisoning by grooming ([HLT-27](ideas/body-and-mind.md)) · [ ] Drowning sorrows ([MND-08](ideas/body-and-mind.md))
- **Done when:** A festival leaves some dwarves drunk and one in a brawl, and a dry fort shows withdrawal.

#### C09 The tavern night (L)
- **Why:** The tavern should be the fort's drama engine: performers, gambling, tall tales and guests with old feuds.
- **Steps:** [ ] Drunks and bar brawls ([SOC-05](ideas/society.md)) · [ ] Gambling and debt ([SOC-06](ideas/society.md)) · [ ] Tavern keepers and performers ([SOC-30](ideas/society.md); partly: bards on the stage, dwarves play instruments, 1ff31da; no keeper) · [ ] Tall tales and liars ([SOC-19](ideas/society.md)) · [ ] Visitors bring old feuds ([SOC-14](ideas/society.md)) · [ ] Taverns with paying guests ([SOC-31](ideas/society.md)) · [ ] Residents and citizens ([SOC-32](ideas/society.md)) · [ ] Arena and fight pit ([SOC-26](ideas/society.md); partly: Arena mode on the title screen, not in the fort)
- **Done when:** A guest with a feud meets their enemy in the tavern and a fight breaks out.

#### C10 Gods of the mountain (L)
- **Why:** Religion is a temple zone and a need to pray; dwarves should have gods, priests and petitions, and the Pious trait something to do.
- **Steps:** [x] Gods with spheres ([MYT-10](ideas/myth-and-depths.md), 965eafb) · [ ] Temples, priests and sermons ([MYT-11](ideas/myth-and-depths.md); partly: temples dedicated to a god, 965eafb; no priests) · [ ] Temple design from the god ([MYT-28](ideas/myth-and-depths.md)) · [ ] Religious petitions ([MYT-12](ideas/myth-and-depths.md)) · [ ] Faith passed on ([MYT-41](ideas/myth-and-depths.md)) · [ ] Cursing the gods ([MYT-27](ideas/myth-and-depths.md)) · [ ] Divination shrine ([MYT-24](ideas/myth-and-depths.md)) · [ ] Desecration and divine curses ([MYT-13](ideas/myth-and-depths.md); partly: broken taboos, blood on the temple floor among them, bring a god's curse, 795962e; no vampires or werebeasts) · [ ] Beast cults ([MYT-14](ideas/myth-and-depths.md)) · [ ] Answered prayers ([MYT-15](ideas/myth-and-depths.md))
- **Done when:** A temple dedicated to one god draws its worshippers, and a pious overseer grants a priest's petition.

#### C11 Table and tankard (M)
- **Why:** Food with names: a great cook's dish and a famous brew remembered, feasts that matter.
- **Steps:** [ ] Named drinks and vintages ([AGR-05](ideas/economy.md)) · [ ] Signature recipes ([AGR-06](ideas/economy.md)) · [ ] Spoilage by heat ([AGR-31](ideas/economy.md)) · [ ] Fishery ([AGR-20](ideas/economy.md))
- **Done when:** A legendary cook's dish is named and remembered in the chronicle.

#### C12 Breakdowns (M)
- **Why:** The spectacular failures: tantrums with limits, and dark moods that make terrible artifacts.
- **Steps:** [ ] Tantrums with limits ([MND-32](ideas/body-and-mind.md)) · [ ] Darker moods ([MND-14](ideas/body-and-mind.md))
- **Done when:** A dwarf in a dark mood makes a macabre artifact, and the fort is shaken by it.

### Milestone 4: the world comes to you (v0.6)

Goal: the world map is alive. What happens out there reaches the fort, and the fort reaches back.

#### B01 Scouts and armies on the world map (M)
- **Why:** sieges appear from nowhere. Seeing an army march across the map is a story.
- **Steps:** [ ] armies and caravans move between sites on the world map, drawn on it · [ ] a
  scout warns days before a siege, with its size and leader (partly: a day ahead, S03; half a day
  more from a watchtower, ad88122; a season ahead if a captive talked) · [ ] roads and tracks between
  sites drawn on the world map
- **Done when:** a siege is announced days ahead and the army can be seen approaching.

#### B02 Diplomacy (M)
- **Why:** neighbours should be people, not just a siege timer.
- **Steps:** [ ] envoys arrive with demands (tribute, an artifact, a prisoner) or offers
  (peace, trade) · [ ] paying or refusing changes the next siege · [ ] the overseer decides by its
  traits
- **From the Bay 12 review:** [ ] Envoys with demands ([NOB-15](ideas/society.md)) · [ ] Parley at the gate ([NOB-16](ideas/society.md)) · [ ] Elves judge your forestry ([NOB-13](ideas/society.md)) · [ ] Treating envoys badly ([NOB-12](ideas/society.md)) · [ ] Tribute and taxes to the homeland ([NOB-17](ideas/society.md)) · [ ] Liaison negotiations ([NOB-58](ideas/society.md)) · [ ] Trade that dies off ([NOB-54](ideas/society.md)) · [ ] First contact ([NOB-56](ideas/society.md)) · [ ] Messenger missions ([NOB-57](ideas/society.md)) · [ ] Parley at the gate ([OVR-01](ideas/world-and-overseer.md)) · [x] Prisoners of war ([OVR-12](ideas/world-and-overseer.md), 6c8fa4d) · [ ] Banish a dwarf ([OVR-08](ideas/world-and-overseer.md)) · [ ] Take or refuse the barony ([OVR-09](ideas/world-and-overseer.md)) · [ ] Petitions ([OVR-11](ideas/world-and-overseer.md)) · [ ] The crown's levy ([OVR-14](ideas/world-and-overseer.md)) · [ ] Pleas for help ([OVR-15](ideas/world-and-overseer.md)) · [ ] Conduct has consequences ([OVR-13](ideas/world-and-overseer.md))
- **Done when:** paying tribute delays a siege and refusing brings it sooner.

#### F07 Monster hunters and performers (S)
- **Why:** finishes the wandering heroes (bards and warriors are done).
- **Steps:** [ ] monster hunters come for a beast in your caverns and may die trying · [ ]
  performers dance and tell stories in the tavern, and the stories come from the legends (partly:
  bards sing the legends and the fort's own works, fce2b3a; no dancers or storytellers)
- **From the Bay 12 review:** [ ] Fame abroad ([OUT-12](ideas/world-and-overseer.md)) · [ ] Tall tales from returning squads ([OUT-05](ideas/world-and-overseer.md)) · [ ] Monster slayers ([OUT-16](ideas/world-and-overseer.md)) · [ ] Questers come for your artifacts ([OUT-13](ideas/world-and-overseer.md)) · [ ] Recover stolen artifacts ([OUT-06](ideas/world-and-overseer.md)) · [ ] Treasure trails ([OUT-07](ideas/world-and-overseer.md)) · [ ] Talk of the fort's buildings ([OUT-28](ideas/world-and-overseer.md))
- **Done when:** a hunter arrives when a cave horror lives and goes after it.

#### W25 Getting around big maps (S)
- **Why:** maps grow to 320×256 now; scrolling there is slow.
- **Steps:** [ ] a fortress mini-map (toggle) with the view outlined, clickable · [ ] saved
  camera spots (Ctrl+1-9 to save, 1-9 to jump) · [ ] typed world names and favourite seeds
- **From the Bay 12 review:** [ ] Recenter hotkeys and fort minimap ([UI-31](ideas/world-and-overseer.md)) · [ ] Places and objects lists ([UI-21](ideas/world-and-overseer.md)) · [ ] Map notes ([UI-17](ideas/world-and-overseer.md)) · [ ] Depth display ([UI-19](ideas/world-and-overseer.md)) · [ ] Quick hops ([UI-34](ideas/world-and-overseer.md)) · [ ] Stocks window and search ([UI-28](ideas/world-and-overseer.md))
- **Done when:** clicking the mini-map jumps the view there.

#### B15 The caravan remembers (M)
- **Why:** Killing every merchant changes nothing today, and the overseer trades blind to its traits; trade should have consequences.
- **Steps:** [ ] Caravans bring what you lack ([ECO-03](ideas/economy.md)) · [ ] Haggling explained ([ECO-06](ideas/economy.md)) · [ ] Starving dwarves raid the caravan ([ECO-01](ideas/economy.md)) · [ ] Harming merchants has fallout ([ECO-02](ideas/economy.md)) · [ ] Trade that dies off ([ECO-07](ideas/economy.md)) · [ ] Wary caravans ([ECO-08](ideas/economy.md)) · [ ] Neighbours judge your conduct ([ECO-05](ideas/economy.md)) · [ ] Export bans ([ECO-10](ideas/economy.md)) · [ ] Nobility drives trade ([ECO-09](ideas/economy.md))
- **Done when:** Robbing a caravan sours that civilization and its next caravan doesn't come.

#### B16 Enemies with faces (L)
- **Why:** Armies should want things and remember wrongs: leaders with pets, grudges stated with reasons, armies you can watch march.
- **Steps:** [ ] Meet the enemy leader ([SIE-06](ideas/war.md)) · [ ] Beasts of war and leaders' pets ([SIE-36](ideas/war.md)) · [ ] Grudges with reasons ([SIE-05](ideas/war.md)) · [ ] Parley at the gate ([SIE-09](ideas/war.md)) · [ ] Carried off alive ([SIE-24](ideas/war.md)) · [ ] Evil armies with character ([SIE-32](ideas/war.md); partly: peoples made by villains besiege with their maker's gift, 0ccdbeb) · [ ] Armies you can watch coming ([SIE-33](ideas/war.md))
- **Done when:** A siege leader names the wrong that brought them, and their army is seen marching across the world map.

#### B17 The world turns (L)
- **Why:** The world should move while the fort lives: a yearly history pass, wars starting and ending, news arriving.
- **Steps:** [ ] The world turns each year ([WLD-05](ideas/world-and-overseer.md)) · [ ] Wars declared and ended in play ([WLD-06](ideas/world-and-overseer.md)) · [ ] News and rumours reach the fort ([WLD-07](ideas/world-and-overseer.md)) · [ ] Towns fall and rise ([WLD-19](ideas/world-and-overseer.md)) · [ ] Alliances against doom ([WLD-20](ideas/world-and-overseer.md)) · [ ] Refugees and bandit camps ([WLD-21](ideas/world-and-overseer.md)) · [ ] A fully living world ([WLD-32](ideas/world-and-overseer.md))
- **Done when:** After three years the legends show wars and deaths elsewhere that the fort heard about.

#### B18 Missions that matter (M)
- **Why:** Expeditions should have stakes and aftermath: captives left behind, rescues, retaliation.
- **Steps:** [ ] Expeditions leave captives behind ([OUT-01](ideas/world-and-overseer.md)) · [ ] Rescue missions ([OUT-02](ideas/world-and-overseer.md)) · [ ] Retaliation for raids ([OUT-03](ideas/world-and-overseer.md)) · [ ] Pillage depletion ([OUT-21](ideas/world-and-overseer.md)) · [ ] Routes on the map ([OUT-22](ideas/world-and-overseer.md)) · [ ] Coming home ([OUT-18](ideas/world-and-overseer.md))
- **Done when:** A beaten expedition leaves a dwarf captive, and a later one can bring them home.

#### B19 Faces at the gate (M)
- **Why:** Migrants should be people with pasts: why they came, who they knew, what they heard.
- **Steps:** [ ] Death drives migrants away ([SOC-18](ideas/society.md)) · [ ] Turning migrants away ([SOC-44](ideas/society.md)) · [ ] Ask for the workers you need ([SOC-45](ideas/society.md)) · [ ] Migrants with a past ([SOC-16](ideas/society.md)) · [ ] Survivors of fallen forts return ([SOC-17](ideas/society.md)) · [ ] Migrants with baggage ([SOC-53](ideas/society.md)) · [ ] Persecuted kin ([SOC-46](ideas/society.md))
- **Done when:** A migrant arrives with a reason, and someone in the fort already knows them.

#### B20 Pests and predators (L)
- **Why:** The farm should be a front line: vermin in the stores, pets that eat pets, fish that run out.
- **Steps:** [ ] Pets eat pets ([AGR-01](ideas/economy.md)) · [ ] Vermin in stores and fields ([AGR-04](ideas/economy.md); partly: vermin eat loose food and cats catch them, 4c95ef8; food doesn't draw them) · [ ] Cats fish ([AGR-14](ideas/economy.md)) · [ ] Fish run out ([AGR-10](ideas/economy.md)) · [ ] Livestock raids ([AGR-02](ideas/economy.md)) · [x] Wildlife populations ([AGR-18](ideas/economy.md), 41057bf; predators hunt prey 4c95ef8) · [ ] Logging angers the wild ([AGR-09](ideas/economy.md))
- **Done when:** A hoarded food store draws vermin, and a fort with cats loses less.

#### B21 Snatchers and thieves (M)
- **Why:** Small, sneaky threats make stories between sieges: child snatchers, kobolds, gremlins, thieving animals.
- **Steps:** [ ] Baby snatchers ([CRE-04](ideas/myth-and-depths.md)) · [ ] Gremlins ([CRE-11](ideas/myth-and-depths.md)) · [ ] Proper kobolds ([CRE-12](ideas/myth-and-depths.md)) · [ ] Scavengers ([CRE-06](ideas/myth-and-depths.md); partly: carrion animals come to carcasses and battlefields, 4c95ef8) · [ ] Thieving animals ([CRE-26](ideas/myth-and-depths.md)) · [ ] Night trolls ([CRE-14](ideas/myth-and-depths.md))
- **Done when:** A child is taken at night, and the fort can try to get them back.

#### B22 Beasts of legend (L)
- **Why:** Monsters should have names, habits and powers that the fort learns about, and hunters who come for them.
- **Steps:** [x] Forgotten beasts with powers ([CRE-05](ideas/myth-and-depths.md), 8dd51f6) · [ ] Man-eaters become legends ([CRE-08](ideas/myth-and-depths.md)) · [ ] Disturbed lairs ([CRE-09](ideas/myth-and-depths.md); partly: dens on the map, and a squad sent to clear one is met by its beasts, c3e0eec) · [ ] Cyclic beasts ([CRE-10](ideas/myth-and-depths.md)) · [ ] Monster hunters ([CRE-07](ideas/myth-and-depths.md)) · [ ] Flying snatchers ([CRE-13](ideas/myth-and-depths.md)) · [ ] Dragon lairs and hoards ([CRE-18](ideas/myth-and-depths.md)) · [ ] Hydras fight on ([CRE-21](ideas/myth-and-depths.md)) · [ ] Megabeasts take fallen forts ([CRE-19](ideas/myth-and-depths.md)) · [ ] Generated megabeasts that breed and bargain ([CRE-25](ideas/myth-and-depths.md))
- **Done when:** A cave horror's generated power changes how it fights, and the fort learns it after the first clash.

#### B23 The restless dead (L)
- **Why:** Death in the fort shouldn't be quiet: ghosts with personalities, horror at risen kin, tomb curses.
- **Steps:** [ ] Richer ghosts ([MYT-01](ideas/myth-and-depths.md)) · [x] Horror at undead kin ([MYT-02](ideas/myth-and-depths.md), d66ee5c) · [ ] Dormant dead ([MYT-17](ideas/myth-and-depths.md)) · [ ] Mummies and tomb curses ([MYT-08](ideas/myth-and-depths.md)) · [ ] Undead from parts ([MYT-16](ideas/myth-and-depths.md)) · [ ] Evil with a cause, evil purged ([MYT-09](ideas/myth-and-depths.md)) · [ ] Spreading evil land ([MYT-20](ideas/myth-and-depths.md)) · [ ] Secrets of death in the fort ([MYT-19](ideas/myth-and-depths.md)) · [ ] Necromancer escalation ([MYT-18](ideas/myth-and-depths.md); partly: necromancers make new peoples in the history, 0ccdbeb)
- **Done when:** A drowned dwarf's ghost haunts the dwarf who pulled the lever.

#### B10 Why you came (M)
- **Why:** Starting scenarios with a backstory, each pairing naturally with an overseer style.
- **Steps:** [ ] Why you came ([OTH-02](ideas/world-and-overseer.md)) · [ ] Starting scenarios ([OTH-01](ideas/world-and-overseer.md))
- **Done when:** A mining-company embark starts with different dwarves, goods and goals than a military citadel.

#### B24 A wider market (M)
- **Why:** The world's trade should reach the fort: more peoples, goods that follow the world map, a yearly fair.
- **Steps:** [ ] Trade agreements ([ECO-21](ideas/economy.md)) · [ ] Messengers open trade ([ECO-22](ideas/economy.md)) · [ ] Local prices ([ECO-23](ideas/economy.md)) · [ ] Tribute through caravans ([ECO-19](ideas/economy.md))
- **Done when:** A caravan brings what its home region produces, and prices move with what the fort sold last year.

### Milestone 5: war in earnest (v0.7)

Goal: sieges are never the same twice. Invaders adapt and the fort adapts back; soldiers have nerve, routines and legends.

#### A01 Sieges with surprises (M)
- **Why:** Sieges come on the first day of a season and play out alike; they should vary, and the fort should answer each one.
- **Steps:** [ ] Unpredictable arrival ([SIE-02](ideas/war.md)) · [ ] Ambush vs open siege ([SIE-42](ideas/war.md); partly: now and then an army comes unseen, 537cc17) · [x] Freeing the caged ([SIE-04](ideas/war.md), 5cd7dd7) · [x] Victory cheer and feast ([SIE-03](ideas/war.md), c1a411a) · [ ] Traps jammed with bodies ([SIE-19](ideas/war.md)) · [ ] Drowning the invaders ([SIE-20](ideas/war.md))
- **Done when:** Sieges arrive at varied times and from varied edges, and a broken siege lifts the fort's mood.

#### A02 Blood and nerve (L)
- **Why:** Fights should be decided by courage as well as hit points: routs, rallies, levies and prisoners.
- **Steps:** [ ] Morale, fear and rout ([MIL-06](ideas/war.md); partly: armies rout when the leader falls or half are caged; beasts lose nerve, 8cc661b) · [ ] A leader's fall cuts both ways ([MIL-09](ideas/war.md)) · [ ] Rallying charge ([MIL-10](ideas/war.md)) · [ ] Emergency levy and martial law ([MIL-07](ideas/war.md)) · [ ] Cornered civilians fight back ([MIL-08](ideas/war.md)) · [ ] Bystanders choose sides ([MIL-17](ideas/war.md)) · [ ] Yielding and prisoners of war ([MIL-16](ideas/war.md)) · [ ] Mutiny and desertion ([MIL-15](ideas/war.md))
- **Done when:** A squad routs when its leader falls, and a broken goblin can yield and be taken prisoner.

#### W20 Attributes and skill decay (M)
- **Why:** Strength, agility and toughness should set dwarves apart in a fight; skills fade without use.
- **Steps:** [ ] Attributes and skill rust ([MIL-37](ideas/war.md)) · [ ] Attributes that grow and rust ([HLT-32](ideas/body-and-mind.md))
- **Done when:** Two dwarves with the same skill but different strength fight differently in the arena.

#### A03 Legends of the blade (M)
- **Why:** Battles and weapons should be remembered: named weapons, kill tallies, veterans with reputations.
- **Steps:** [ ] Named battles and a fight log ([MIL-12](ideas/war.md); partly: named battles, 15be5b2; a fight log only in the arena) · [ ] Weapons that earn names ([MIL-11](ideas/war.md)) · [ ] Per-dwarf kill records ([MIL-67](ideas/war.md)) · [ ] Revenge and nemesis ([MIL-13](ideas/war.md)) · [ ] One appointed champion ([MIL-44](ideas/war.md)) · [ ] Favourite weapons ([MIL-41](ideas/war.md))
- **Done when:** A weapon that has killed a named enemy gets a name and a history.

#### A04 Squad life (L)
- **Why:** A military that runs on a schedule: routines, patrols, ammo pools, drills and discipline.
- **Steps:** [ ] Squad routines and alerts ([MIL-26](ideas/war.md)) · [ ] Patrol routes ([MIL-27](ideas/war.md)) · [ ] Defend a burrow ([MIL-28](ideas/war.md)) · [ ] Gear off duty ([MIL-29](ideas/war.md)) · [ ] Squad ammo pools ([MIL-30](ideas/war.md)) · [ ] Saved uniforms by kind ([MIL-58](ideas/war.md)) · [x] Teaching and drills ([MIL-38](ideas/war.md)) (b90bbd1) · [x] Sparring accidents ([MIL-39](ideas/war.md)) (b90bbd1) · [ ] Sergeant's discipline ([MIL-40](ideas/war.md))
- **Done when:** A squad keeps a monthly routine and draws bolts from its own pool.

#### B06 Siege AI (L)
- **Why:** A raised bridge makes a fort untouchable; invaders should dig, climb, build and ram, and learn from failed assaults.
- **Steps:** [ ] Wall-breakers ([SIE-10](ideas/war.md)) · [ ] Reinforced walls ([SIE-40](ideas/war.md)) · [ ] Climbers and jumpers ([SIE-13](ideas/war.md)) · [ ] Siege builders ([SIE-12](ideas/war.md)) · [ ] Sappers ([SIE-11](ideas/war.md)) · [ ] Invaders learn from losses ([SIE-14](ideas/war.md)) · [ ] Cut-off leaders adapt ([SIE-39](ideas/war.md)) · [ ] Battering rams ([SIE-15](ideas/war.md)) · [ ] Feints and fall-backs ([SIE-34](ideas/war.md))
- **Done when:** An army facing a raised bridge digs or builds its way in.

#### A05 Engines of war (L)
- **Why:** A siege workshop for both sides: ballistae and catapults on the walls, rams at the gate.
- **Steps:** [ ] Bolt thrower ([SIE-16](ideas/war.md)) · [ ] Catapult with shattering stones ([SIE-17](ideas/war.md)) · [ ] Siege crews ([SIE-54](ideas/war.md)) · [ ] Attackers' siege engines ([SIE-55](ideas/war.md)) · [ ] Long sieges and siege camps ([SIE-22](ideas/war.md))
- **Done when:** A ballista on the wall kills a ram crew before it reaches the gate.

#### A06 When all is lost (M)
- **Why:** Last resorts keep losing fun: floodgates as weapons, martial law, surrender and occupation.
- **Steps:** [ ] Floodgates as weapons ([OVR-17](ideas/world-and-overseer.md)) · [ ] Martial law and the last stand ([OVR-07](ideas/world-and-overseer.md)) · [ ] Surrender and occupation ([OVR-20](ideas/world-and-overseer.md))
- **Done when:** An overseer facing defeat floods the entrance hall, or surrenders and the fort is occupied.

#### B03 Armies abroad (L)
- **Why:** The fort should reach out with force: raids, tribute, rescues, conquest that makes it a barony.
- **Steps:** [ ] Armies abroad: raid, raze, loot ([MIL-18](ideas/war.md)) · [ ] Military tactics skill ([MIL-45](ideas/war.md)) · [ ] Rescue missions ([MIL-19](ideas/war.md)) · [ ] Recover stolen treasures ([MIL-20](ideas/war.md)) · [ ] Strike at enemy leaders ([MIL-21](ideas/war.md)) · [ ] Field armies as a shield ([MIL-23](ideas/war.md)) · [ ] Conquest, tribute, barony ([MIL-61](ideas/war.md)) · [ ] The crown's levy ([MIL-22](ideas/war.md))
- **Done when:** A raid on a goblin site brings back loot and a captive, and their retaliation follows.

### Milestone 6: deep fortress (v0.8)

Goal: the deep places are dangerous and worth it.

#### F05 Disasters (S–M)
- **Why:** cave-ins are done; the rest make a fort's history.
- **Steps:** [ ] earthquakes crack walls and open new caverns · [ ] the spring melt floods the
  lowest halls · [ ] the magma sea surges
- **Done when:** a flood fills a low hall in spring and dwarves flee it.

#### B13 Lighting (M)
- **Why:** the caverns should be dark and frightening.
- **Steps:** [ ] dark caverns and lit halls; torches and lamps · [ ] ambushes from the dark ·
  [ ] dwarves dislike working in the dark
- **From the Bay 12 review:** [ ] Fire as tool and threat ([ENG-05](ideas/economy.md)) · [ ] Light and darkness ([ENG-25](ideas/economy.md)) · [ ] Hearths and chimneys ([ENG-50](ideas/economy.md)) · [ ] Fluid heat ([ENG-52](ideas/economy.md))
- **Done when:** a creature in an unlit cavern can't be seen until a dwarf is close.

#### W26 Water and magma works (M)
- **Why:** pumps, water wheels and magma channels are DF's great engineering projects.
- **Steps:** [ ] screw pumps lift water or magma a level · [ ] aquifer tools: pumping a shaft dry,
  and a warning before embarking on a heavy aquifer · [ ] magma creatures in the sea
- **From the Bay 12 review:** [ ] Wells and cisterns inside ([ENG-18](ideas/economy.md); partly: cistern.rs, 0b0db40, but no fort finds a site) · [ ] Warm and damp warnings, aquifer tools ([ENG-15](ideas/economy.md)) · [ ] Screw pumps ([ENG-21](ideas/economy.md)) · [ ] Magma works ([ENG-27](ideas/economy.md)) · [ ] Fresh, salt and stagnant water ([ENG-37](ideas/economy.md)) · [ ] More water sources ([ENG-38](ideas/economy.md)) · [ ] Power ([ENG-28](ideas/economy.md)) · [ ] Eruptions and surges ([ENG-26](ideas/economy.md)) · [ ] Volcanoes refill ([ENG-40](ideas/economy.md))
- **Done when:** a pump stack drains a flooded shaft.

#### D01 The living caverns (M)
- **Why:** The caverns should react: noise from the fort irritates them, cavern peoples raid from below, things hide in the dark.
- **Steps:** [ ] Cavern irritation ([DIG-01](ideas/myth-and-depths.md)) · [ ] Sounds from the deep ([DIG-09](ideas/myth-and-depths.md)) · [ ] Endless cavern life ([DIG-13](ideas/myth-and-depths.md)) · [ ] Cavern peoples ([DIG-03](ideas/myth-and-depths.md)) · [ ] Webs ([DIG-14](ideas/myth-and-depths.md); partly: cave horrors' webs hold a dwarf fast, 8dd51f6) · [ ] Cave fliers ([DIG-15](ideas/myth-and-depths.md); partly: cave bats and lanternmoths, 4c95ef8; no swarms) · [ ] Gradual reveal and the unknown ([DIG-17](ideas/myth-and-depths.md)) · [ ] Dark caverns ([DIG-08](ideas/myth-and-depths.md))
- **Done when:** A noisy fort draws raids from the caverns that a quiet one avoids.

#### D02 Digging too deep (L)
- **Why:** The deep metal should tempt greedy and delving overseers, with warnings before and a changed world after.
- **Steps:** [ ] Warnings from below ([DIG-02](ideas/myth-and-depths.md)) · [ ] Deep metal greed ([DIG-05](ideas/myth-and-depths.md)) · [ ] The underworld ([DIG-04](ideas/myth-and-depths.md)) · [ ] Digging too deep in history ([DIG-28](ideas/myth-and-depths.md)) · [ ] Deeper, stranger layers ([DIG-27](ideas/myth-and-depths.md))
- **Done when:** A Delver that mines out the deep vein opens the way for something terrible.

#### D03 Farms and forests below (M)
- **Why:** Cavern soil and fungus forests should pull the fort into danger for wood and food.
- **Steps:** [ ] Rich cavern soil ([DIG-10](ideas/myth-and-depths.md)) · [ ] Fungus forests ([DIG-18](ideas/myth-and-depths.md)) · [ ] Chasm disposal ([DIG-19](ideas/myth-and-depths.md)) · [ ] Underground waters ([DIG-26](ideas/myth-and-depths.md)) · [ ] Magma dwellers ([DIG-20](ideas/myth-and-depths.md))
- **Done when:** A fort with no trees on the surface logs a fungus forest in the caverns.

#### D04 Hold up the roof (L)
- **Why:** Cave-ins are the only disaster; supports, collapse rules and engineering should make wide halls a choice.
- **Steps:** [ ] Supports and collapse ([ENG-24](ideas/economy.md)) · [ ] Size-limited passages ([ENG-22](ideas/economy.md)) · [ ] Secret doors and escape tunnels ([ENG-23](ideas/economy.md)) · [ ] Wear on stonework ([ENG-51](ideas/economy.md)) · [ ] Moving sections ([ENG-30](ideas/economy.md))
- **Done when:** A support lets a wide hall stand that would otherwise come down.

#### D05 Sickness in the halls (L)
- **Why:** Plague, vermin and poison as a slow-burning crisis the overseer must quarantine.
- **Steps:** [ ] Contagious disease ([HLT-06](ideas/body-and-mind.md)) · [ ] Vermin-borne plague ([HLT-25](ideas/body-and-mind.md)) · [x] Retching at carnage ([HLT-11](ideas/body-and-mind.md), afba754: the squeamish retch at the worst sights, a blow to the gut makes one vomit, and vomit is tracked about) · [ ] Hygiene and baths ([HLT-30](ideas/body-and-mind.md)) · [ ] Blood and grime ([HLT-31](ideas/body-and-mind.md); partly: blood pools and trails that rain washes and haulers scrub, d28911c; blood sprayed up the walls, bloody footprints, fighters covered in blood until water or rain washes it, bloodied weapons, afba754; no mud or vomit) · [ ] Poisons and venom ([HLT-26](ideas/body-and-mind.md); partly: a cave horror's venom worsens the wound, 8dd51f6) · [ ] Herbal remedies ([HLT-37](ideas/body-and-mind.md)) · [ ] Strange afflictions ([HLT-35](ideas/body-and-mind.md))
- **Done when:** A sickness spreads through the dining hall until the overseer quarantines it.

#### D06 Harsh world (M)
- **Why:** The surface and the deep should both hurt: sun sickness, cold, heat, bad air.
- **Steps:** [ ] Sun sickness ([HLT-05](ideas/body-and-mind.md)) · [ ] Surviving cave-ins ([HLT-12](ideas/body-and-mind.md)) · [ ] Burning dwarves react ([HLT-13](ideas/body-and-mind.md)) · [ ] Falls that hurt ([HLT-23](ideas/body-and-mind.md)) · [ ] Cold injuries and frostbite ([HLT-22](ideas/body-and-mind.md)) · [ ] Blind in the dark ([HLT-36](ideas/body-and-mind.md))
- **Done when:** Dwarves raised underground falter in the sun until they grow used to it.

#### D07 Marked for life (L)
- **Why:** Every fight should leave a visible trace that changes the dwarf: scars, lost parts, prosthetics.
- **Steps:** [ ] Scars, brands and lost teeth ([HLT-08](ideas/body-and-mind.md); partly: teeth fly, d28911c; the Life portrait shows a head scar, f031dd9) · [ ] Pain ([HLT-18](ideas/body-and-mind.md)) · [ ] Prosthetics ([HLT-16](ideas/body-and-mind.md)) · [ ] Old wounds from history ([HLT-14](ideas/body-and-mind.md)) · [ ] Head injuries change people ([HLT-15](ideas/body-and-mind.md)) · [ ] Old age decline ([HLT-17](ideas/body-and-mind.md)) · [ ] Beards and hair ([HLT-34](ideas/body-and-mind.md); partly: the portrait's beards grey with age, f031dd9; the wizard turns one blue)
- **Done when:** A veteran carries scars on their sheet and an iron hand that lets them fight again.

#### D08 The rest of the workshop row (L)
- **Why:** Glass, kilns, jewellers and melting complete DF's industry.
- **Steps:** [ ] Melting and recycling ([CRF-10](ideas/economy.md)) · [ ] More metals and gems ([CRF-44](ideas/economy.md)) · [ ] Whole-carcass butchery ([CRF-25](ideas/economy.md); partly: each animal's products, wool, feathers, ivory, venom, 4c95ef8) · [ ] Leather done right ([CRF-31](ideas/economy.md)) · [ ] Kiln, ceramics, lime ([CRF-45](ideas/economy.md)) · [ ] Glass furnace ([CRF-46](ideas/economy.md)) · [ ] Oil, potash, lye ([CRF-32](ideas/economy.md)) · [x] Musical instruments ([CRF-15](ideas/economy.md), 1ff31da) · [ ] Toys and games ([CRF-14](ideas/economy.md))
- **Done when:** The overseer melts down captured goblin weapons into bars.

#### B11 Logistics (L)
- **Why:** Fewer trips, then wheelbarrows and carts: hauling one item at a time limits every fort.
- **Steps:** [ ] Multi-item hauling ([ECO-46](ideas/economy.md)) · [ ] Stockpile links ([ECO-36](ideas/economy.md)) · [ ] Quality filters and tidying ([ECO-37](ideas/economy.md)) · [ ] Wheelbarrows ([ECO-47](ideas/economy.md)) · [ ] Burrow logistics ([ECO-48](ideas/economy.md)) · [ ] Minecarts and hauling routes ([ECO-29](ideas/economy.md))
- **Done when:** A hauler carries several items at once and a minecart line feeds a workshop.

#### D09 The barnyard (L)
- **Why:** Animals that eat and give: grazing, milk, wool, eggs, trainers, bees.
- **Steps:** [ ] Grazing and hunger ([AGR-11](ideas/economy.md)) · [ ] Eggs, nests and hatchlings ([AGR-12](ideas/economy.md); partly: kept hens lay eggs, 4c95ef8) · [ ] Wool, milk and cheese ([AGR-16](ideas/economy.md); partly: sheep shed wool, 4c95ef8) · [ ] Beekeeping ([AGR-13](ideas/economy.md)) · [ ] Herd management ([AGR-25](ideas/economy.md)) · [ ] Breeding needs contact ([AGR-26](ideas/economy.md)) · [ ] Training knowledge per species ([AGR-07](ideas/economy.md)) · [ ] Animal trainers and cues ([AGR-30](ideas/economy.md)) · [ ] War beasts ([AGR-17](ideas/economy.md))
- **Done when:** A pasture keeps goats fed and milked through the summer.

#### B12 Living farms (L)
- **Why:** Soil, water, weeds and the spring flood should make farming a craft.
- **Steps:** [ ] Soil depth ([AGR-03](ideas/economy.md)) · [ ] Moisture and irrigation ([AGR-36](ideas/economy.md)) · [ ] Fertiliser and fallow ([AGR-37](ideas/economy.md)) · [ ] Flood farming ([AGR-08](ideas/economy.md)) · [ ] Orchards and growths ([AGR-21](ideas/economy.md)) · [ ] Underground wood ([AGR-22](ideas/economy.md)) · [ ] Multi-tile trees and felling ([AGR-24](ideas/economy.md))
- **Done when:** An irrigated field yields more than a dry one, and weeds take an untended one.

#### D10 Wants and pastimes (L)
- **Why:** DF has about thirty needs to our seven: small reasons to be happy or not.
- **Steps:** [ ] More needs ([MND-24](ideas/body-and-mind.md)) · [ ] Wider likes and dislikes ([MND-42](ideas/body-and-mind.md)) · [ ] Food that matters ([MND-43](ideas/body-and-mind.md)) · [ ] Hobbies ([MND-47](ideas/body-and-mind.md)) · [ ] Hip flasks ([MND-44](ideas/body-and-mind.md)) · [ ] Smoking circles ([MND-45](ideas/body-and-mind.md)) · [ ] Toys and play ([MND-46](ideas/body-and-mind.md)) · [ ] Admiring displays ([MND-36](ideas/body-and-mind.md); partly: dwarves admire statues and crafts of what they lived through, f6ad60c) · [ ] Pride in masterworks ([MND-35](ideas/body-and-mind.md)) · [ ] Life dreams ([MND-10](ideas/body-and-mind.md)) · [ ] Values and more facets ([MND-25](ideas/body-and-mind.md))
- **Done when:** A dwarf who admires art and never sees any is unhappier than one who doesn't care.

#### D11 Omens and skies (M)
- **Why:** The sky as drama: eclipses, prophecies, storms, thin ice.
- **Steps:** [x] Sky events ([WLD-03](ideas/world-and-overseer.md), 795962e: eclipse, blood moon, falling star; the anxious fret, the devout read a sign) · [ ] Prophecies ([WLD-08](ideas/world-and-overseer.md); partly: omens and dreams that come true, 795962e, f293b20) · [ ] Thin ice that breaks ([WLD-04](ideas/world-and-overseer.md)) · [ ] Wild weather, more of it ([WLD-11](ideas/world-and-overseer.md)) · [ ] Deep snow and rain puddles ([WLD-16](ideas/world-and-overseer.md))
- **Done when:** An eclipse unsettles the fort and the pious read it as an omen.

### Milestone 7: dread (v0.9)

Goal: the worst threats come from inside. Night creatures, spies and curses make the fort suspicious of itself.

#### N01 Things in the night (M)
- **Why:** Night creatures with known banes: werebeasts under the moon, a vampire hiding among the migrants, omens before them.
- **Steps:** [x] Omens ([MYT-05](ideas/myth-and-depths.md), 795962e) · [x] Banes and weaknesses ([MYT-06](ideas/myth-and-depths.md), the thing in the night, e5c7492) · [ ] Werebeast raid and moon curse ([MYT-03](ideas/myth-and-depths.md)) · [ ] Secret vampire citizen ([MYT-04](ideas/myth-and-depths.md)) · [ ] Bogeymen ([MYT-07](ideas/myth-and-depths.md)) · [ ] Sensing evil ([MYT-26](ideas/myth-and-depths.md)) · [ ] Charms and wards ([MYT-25](ideas/myth-and-depths.md); partly: a warding engraving can be the thing in the night's bane, e5c7492)
- **Done when:** A vampire among the migrants drains sleepers until the fort works out who it is.

#### B04 Enemies within (L)
- **Why:** Villains, spies and traitors among the citizens, uncovered by interrogation.
- **Steps:** [ ] Questers in the tavern ([NOB-29](ideas/society.md)) · [ ] Secret vampire citizen ([NOB-24](ideas/society.md)) · [ ] Spies and double agents ([NOB-25](ideas/society.md)) · [ ] Interrogation ([NOB-21](ideas/society.md); partly: a caged besieger is questioned and lets slip the next siege) · [ ] Traitorous advisor ([NOB-28](ideas/society.md)) · [ ] Villain networks ([NOB-26](ideas/society.md)) · [ ] Assassination plots ([NOB-27](ideas/society.md)) · [ ] Counterintelligence screen ([NOB-61](ideas/society.md)) · [ ] Exile returns ([NOB-33](ideas/society.md)) · [ ] Traitorous advisors ([OVR-23](ideas/world-and-overseer.md))
- **Done when:** A spy is caught and interrogated, and names the plot behind them.

#### N02 Shadows at the door (L)
- **Why:** The quiet threats between sieges: assassins, saboteurs, poisoned wells, thieves who strike and flee.
- **Steps:** [ ] Sneaks before armies ([SIE-07](ideas/war.md)) · [ ] Thieves who plan ([SIE-44](ideas/war.md)) · [ ] Wider theft and rustling ([SIE-46](ideas/war.md)) · [ ] Distraction thieves ([SIE-47](ideas/war.md)) · [ ] Baby-snatchers ([SIE-08](ideas/war.md)) · [ ] Bandits, cults and rackets ([SIE-26](ideas/war.md)) · [ ] Assassins and spies ([SIE-27](ideas/war.md)) · [ ] Poisoned water ([SIE-21](ideas/war.md)) · [ ] The exile returns ([SIE-28](ideas/war.md)) · [ ] Kobold raids ([SIE-48](ideas/war.md))
- **Done when:** A saboteur fouls the well and the fort traces it back.

#### N03 Curses and relics (L)
- **Why:** Things that carry a fate: cursed artifacts and sites, powered relics, prophecies, defiled places.
- **Steps:** [ ] Curse framework ([MYT-37](ideas/myth-and-depths.md); partly: a god's curse for a broken taboo, lifted by atoning, 795962e) · [ ] Cursed artifacts ([MYT-21](ideas/myth-and-depths.md)) · [ ] Cursed sites ([MYT-23](ideas/myth-and-depths.md)) · [x] Artifact powers ([MYT-22](ideas/myth-and-depths.md), 644ef99) · [ ] Relics and heirlooms ([MYT-36](ideas/myth-and-depths.md); partly: relics from vaults and ruins carry powers and are coveted, 644ef99; no heirlooms) · [ ] Prophecies ([MYT-32](ideas/myth-and-depths.md); partly: omens and dreams that come true, 795962e, f293b20) · [ ] Places of power ([MYT-30](ideas/myth-and-depths.md)) · [ ] Defilement ([MYT-29](ideas/myth-and-depths.md)) · [ ] Demonic pacts ([MYT-33](ideas/myth-and-depths.md))
- **Done when:** A dwarf who topples a temple's statue in a tantrum is cursed.

### Milestone 8: a world of forts (v1.0)

Goal: a fort's fall is the start of the next story. The world remembers ruins, reigns and roads, and batches play successive forts in one world.

#### P01 Fallen forts live on (L)
- **Why:** A lost fort is only a save slot today; it should become a ruin in the world that a later fort can reclaim, with survivors who come back.
- **Steps:** [ ] Fallen forts enter the world ([WLD-10](ideas/world-and-overseer.md); partly: a fallen fort's sagas go into the world's legends, and `n` on the game-over screen founds a new fort in the same world, which reads them on its Sagas screen; bde720d) · [ ] Time passes between forts ([WLD-25](ideas/world-and-overseer.md)) · [ ] What old forts leave behind ([OUT-11](ideas/world-and-overseer.md)) · [ ] Survivors come back as migrants ([OUT-10](ideas/world-and-overseer.md)) · [ ] Reclaim a fallen fort ([OUT-09](ideas/world-and-overseer.md)) · [ ] Retire and unretire ([OUT-32](ideas/world-and-overseer.md))
- **Done when:** A batch plays three forts in one world, and the third reclaims the first's ruin.

#### P02 Crown and succession (L)
- **Why:** The fort's place in the realm: a monarch, a barony, heirs and claims.
- **Steps:** [ ] Noble deaths matter ([NOB-08](ideas/society.md)) · [ ] Succession in play ([NOB-32](ideas/society.md)) · [ ] Offerings to the ruler ([NOB-44](ideas/society.md)) · [ ] Monarch arrives ([NOB-30](ideas/society.md)) · [ ] Serving the crown ([NOB-31](ideas/society.md)) · [ ] Food riots ([NOB-18](ideas/society.md)) · [ ] Martial law ([NOB-36](ideas/society.md)) · [ ] Pleas for help ([NOB-35](ideas/society.md))
- **Done when:** A happy, rich fort is visited by the monarch, who demands a throne room.

#### P03 Roads and outposts (L)
- **Why:** The fort's reach beyond the map: scouting, messengers, bandits on the roads, outposts.
- **Steps:** [ ] Scouting missions ([OUT-23](ideas/world-and-overseer.md)) · [ ] Messengers between sites ([OUT-31](ideas/world-and-overseer.md)) · [ ] Bandits on the road ([OUT-08](ideas/world-and-overseer.md)) · [ ] Outposts and attached hamlets ([OUT-30](ideas/world-and-overseer.md)) · [ ] Aid missions ([OUT-14](ideas/world-and-overseer.md))
- **Done when:** An outpost on the world map sends goods home and is raided by bandits.

#### P04 Coin and contract (L)
- **Why:** Hired help and debts: mercenaries, rent, loans and the trouble they bring.
- **Steps:** [ ] Mercenaries and monster hunters for hire ([ECO-12](ideas/economy.md)) · [ ] Contracts with part payment ([ECO-25](ideas/economy.md)) · [ ] Patrons fund works ([ECO-26](ideas/economy.md)) · [ ] Protection rackets ([ECO-27](ideas/economy.md)) · [ ] Rooms for rent ([ECO-51](ideas/economy.md)) · [ ] Loans and creditors ([ECO-52](ideas/economy.md)) · [ ] Coins and wages ([ECO-53](ideas/economy.md))
- **Done when:** A mercenary company deserts when its pay runs out.

#### P05 Trade routes of the world (L)
- **Why:** The rest of the world's market: routes, fairs, guild prices and long-distance trade.
- **Steps:** [ ] Caravans on the world map ([ECO-32](ideas/economy.md)) · [ ] Caravan raiders ([ECO-28](ideas/economy.md)) · [ ] Fairs ([ECO-30](ideas/economy.md)) · [ ] Your own caravans ([ECO-31](ideas/economy.md)) · [ ] Hill and deep dwarf partners ([ECO-33](ideas/economy.md)) · [ ] World economy ([ECO-54](ideas/economy.md))
- **Done when:** A trade route opened to a distant site brings a new kind of caravan.

### Other ways to play

Goal: the same world and simulation, seen from other seats.

#### G02 A bigger bestiary (L)
- **Why:** more creatures, each with a way of behaving rather than only new numbers: things
  that make stories.
- **Steps:** [ ] behaviours as reusable parts: ambusher, pack hunter, burrower, flier,
  web-spinner, thief, lair guardian, grazer, scavenger, swimmer (partly: beasts.rs has all but
  the web-spinner, thief and lair guardian, 4c95ef8) · [x] new species by biome
  and depth built from them (see the CRE ideas in [the catalogue](ideas/myth-and-depths.md);
  4c95ef8: 278 animals by land, water and cavern layer) ·
  [x] each new creature appears in the arena first, then in the world (4c95ef8)
- **Done when:** every biome has at least three creatures that behave differently, and each
  can be fought in the arena.

#### G03 Adventurer mode (XL)
- **Why:** take one character into the world for fame, fortune or utter ruin.
- **Needs:** Milestone 4's living world (the world tick, sites as places, news) and Milestone
  8's fallen forts (ruins to explore).
- **Steps:** [x] make a character (race, skills, gear) and start at a site (A1, this commit) · [x] travel the
  world map; enter sites as local maps (towns, ruins, lairs, caves) (sites built on entry from the legends, with site memory: A4 3769753; the road with its travel log, night, weather, encounters, camps and wounds that bleed on the way: A3 31a02ad; saved and loaded whole, the road's encounters and a fallen fort's halls too (Esc menu, F5/F9, autosaves going in and out of sites and at each dawn on the road): A11 a90bc86) · [ ] talk: hear news and
  rumours (`k`: greeted by fame, news, rumours of fallen forts, lairs, ruins and bones marked `!` on the travel map, the old tales as each people tells them this year, directions: A7 dfb75b6, this commit), take jobs, hire companions · [x] fight with the same body and wounds as fortress
  mode; loot; rest and heal (aimed blows and called shots at the eyes and teeth, crossbow bolts seen to fly, binding wounds, the dead's gear taken, the fight's blood drawn on the adventure map: A6, this commit) · [x] fame and infamy in the legends (deeds counted after every act: beasts of legend slain, names won, ruins dared, treasure taken and brought home; townsfolk murdered for infamy; each a chapter of the hero's Wander saga, the great ones legends: A10, this commit); visit or reclaim your old
  forts (visit: a fallen fort entered from its own save, its killer moved in, its saga, journal
  and walls to read, and looting written back to the save after a backup: A5 df2b3d8) · [x] death ends it, and the world remembers (the end page leads with the hero's saga; the hero a figure of the legends, their bones and gear left where they fell for the next hero to find; `n` a fort in this world, `a` another hero; `S` then `R` retires a hero in a friendly town: A10, this commit)
- **Done when:** a character can leave a town, find a ruin from a fallen fort, fight what lives
  there, and come back with its treasure recorded in the legends. (Tested with real travel both ways and real fights, a scripted walker in place of A18's bot; the first
  three heroes die on the road or in the ruin before one comes home:
  `a_hero_walks_home_with_the_treasure_of_a_fallen_fort`.)
- **Measured by:** the adventure bot and its batch (A18, 759fecc): `--adventure-trial --runs N` plays heroes to a fallen fort, a lair and a ruin in turn and reports how they ended, where and to what, with days, kills, loot and fame; `--adventure-trial --compare A B`.

### Later

Bigger systems from Tarn's page, and the epics the review left for after Milestone 8.

| ID | Idea | What it would add | Size | From the catalogue |
|---|---|---|---|---|
| B05 | Artifact politics | Invaders demand or steal artifacts; rumors of where artifacts are; squads sent to recover them | M | [CRF-09](ideas/economy.md), [CRF-04](ideas/economy.md), [CRF-05](ideas/economy.md), [CRF-07](ideas/economy.md) |
| B08 | Taverns, temples, libraries | Performances, guests renting rooms, prayer, books written by dwarves | M |  |
| B09 | Guilds and sects | Skilled dwarves form guilds and religious groups that demand halls and temples, take sides in grudges, and grant perks | M | [SOC-33](ideas/society.md), [SOC-34](ideas/society.md), [SOC-35](ideas/society.md), [SOC-54](ideas/society.md), [SOC-55](ideas/society.md) |
| B14 | Myths and magic | Generated creation myths, deities with powers, wizards and covens, magical invasions | XL | [MYT-58](ideas/myth-and-depths.md), [MYT-46](ideas/myth-and-depths.md), [MYT-45](ideas/myth-and-depths.md), [MYT-47](ideas/myth-and-depths.md), [MYT-48](ideas/myth-and-depths.md), [MYT-51](ideas/myth-and-depths.md), [MYT-56](ideas/myth-and-depths.md) |
| X01 | Lost places below | Underground sites to find or fear: crypts of heroes, ruins that tell their fall, kobold warrens, goblin pits. | L | [DIG-16](ideas/myth-and-depths.md), [DIG-23](ideas/myth-and-depths.md), [DIG-24](ideas/myth-and-depths.md), [DIG-25](ideas/myth-and-depths.md), [DIG-30](ideas/myth-and-depths.md), [DIG-29](ideas/myth-and-depths.md), [DIG-37](ideas/myth-and-depths.md) |
| X02 | Wear, rust and fashion | Clothes and gear that wear out and rust, dyes and fashions. | M | [CRF-30](ideas/economy.md), [CRF-11](ideas/economy.md), [CRF-47](ideas/economy.md), [CRF-48](ideas/economy.md), [CRF-27](ideas/economy.md), [CRF-39](ideas/economy.md) |
| X03 | Clearer orders | Fewer mysteries: why-can't notices, stuck paths, job-done notices, settings and difficulty. | M | [UI-29](ideas/world-and-overseer.md), [UI-11](ideas/world-and-overseer.md), [UI-12](ideas/world-and-overseer.md), [UI-20](ideas/world-and-overseer.md), [UI-26](ideas/world-and-overseer.md), [UI-27](ideas/world-and-overseer.md) |
| X04 | The living wild | The land reacts to the fort: finite populations, the wild's anger at felling, guardian animal people. | L | [CRE-17](ideas/myth-and-depths.md), [CRE-16](ideas/myth-and-depths.md), [CRE-24](ideas/myth-and-depths.md), [CRE-27](ideas/myth-and-depths.md), [CRE-34](ideas/myth-and-depths.md) |

B07 (better traps) went into O04 and B03, B04, B06, B10, B11 and B12 into milestones above.

## Inbox

Ideas not ranked yet, with a line on why. The rest of the old inbox went into the milestones
above; the [idea catalogue](ideas/README.md) holds hundreds more.

- Caged animals could be tamed (W08) and caged goblins interrogated (B04).
- Diagnosis and surgery (DF's fuller medicine, after S02).
- Forbidding by area from the designation menu.
- Murky puddle water should feel worse than river water.
- Adventure mode: explore the generated world. Out of scope until fortress mode is deep.

## Done

- **Sagas** (O09; f9d85c0, bde720d, this commit): the fort links what it already tells into named story arcs (`game/sagas.rs`): pride and fall (sallies won, a host sworn to avenge them, then ruin or a narrow escape), grudges, a hero's rise, omens come true, curses, den and legend hunts, great sieges and the last dwarf standing, each with a cause, escalation, turn and ending quoted with dates and a title hashed from the fort and the moment (no RNG; the golden stories are unchanged). A turn or an ending shows an illuminated card (`ui/saga.rs`): a framed parchment, the title as a banner, a block-letter drop cap, chapters typing in, crimson for doom and gold for glory, with a chime or a dirge; foreshadowing goes in the announcements and the ticker ("The goblins have sworn an oath against Anvilbeard. This will be remembered."). `S` lists the fort's sagas and those of other forts the world remembers; the game-over screen leads with the saga that ended the fort (a fort that fell with none is told as its last deaths). `--autoplay --sagas` prints them: 4.9 a fort over 20 forts × 3 years.
- **Window mode with square tiles** (this commit): `--window` plays in a native window of square cells instead of the terminal, through the same `ui::draw`; text from the system's monospace font scaled to fill the cell (`--font` for another), box lines, blocks, walls, water waves, ramps, trees and floor dots from our own tileset drawn in code, so the map reads as one solid surface; Ctrl +/- or Ctrl+wheel zooms 8..32 px.
- **Sound** (this commit): music, ambience and effects, all synthesized at run time (no sample files): ambience by where the camera is (wind, birds, crickets, rain and thunder, cavern drips and hum, fire, magma, running water); slow modal music on plucked strings and horns over a drone that follows the fort (calm, a jig at feasts, war drums in a siege, a lament after deaths, coloured by the season; its own music on the title, during world generation and in the arena); quiet, rate-limited effects from what's on screen (picks, anvils, fights, roars, the alarm bell, cheers, stingers on big moments, chimes, doors and bridges, menu clicks). `M` mutes anywhere, `O` opens the volumes (saved in `saves/sound.cfg`); growing the map moved to `G`. Behind the default `audio` feature (cpal 0.15.3); silent without a device.
- **Sound for gore and adventure** (d6e6687): the fight is heard for what it does, a few telling sounds a frame at most (a blade going in, a bone or a skull breaking, a cry at a limb lost pitched by the throat (a dragon bellows, a goblin shrieks), a tooth skittering over stone, a body falling, the death rattle, retching, bolts whistling past and thunking home). Adventurers hear their place (a lair breathes and drips, a ruin whistles through its gaps and creaks, a town talks and hammers, the wild sounds of its land by day and night: owls, frogs, crickets), the road (the land's wind, the weather, footsteps) under a sparse travelling theme that darkens at night and in storms, and a fight's drums when foes are near. `--sound-check` plays them all.

- **Arena mode** (G01; 8c2c983, dc56c77, this commit): *Arena* on the title screen sets two sides against each other (any species, dwarves with a fighting skill, counts, weapon, armour and metals, training, bolts) on open ground, a corridor with a door, a cavern, a bridge over a chasm or a walled yard, and plays the bout on its own little map with the siege's blows, blood, severed parts, finishing blows, marksdwarves and battered doors (and the odd fighter knocked off the bridge): any speed, pause, step, follow a fighter, the combat log and the whole report, and a summary (winner, first blood, fighter of the bout, each fighter's wounds, severed parts and kills). `r` reruns it headless N times for win rates, with a table of every rerun; matchups save to `saves/<name>.arena.json`. For fun: ten presets, *surprise me*, and picking a winner before the bout. A duel comes out as the combat.rs arena table has it (80%/30%/35%). The overseer war-games a coming siege in it (its soldiers as kitted against the army as sieges arm it, a dozen bouts): the odds are announced and chronicled, grim odds bring the alert a day early, and after the siege the war-game is judged against what happened.
- **Walls, towers and grand works** (this commit): a grown fort's overseer walls the stair and the fields with a curtain wall (gate towers, battlements, a walkway, an arch and a drawbridge over an always-open gate), raises watchtowers at its corners (a soldier keeps watch; a manned one sees an army half a day sooner), puts its marksdwarves on the towers in an alert to shoot down over the battlements, and (vain, pious, greedy or rich) a wizard's tower with a laboratory where dwarves go to learn; then a pillared portico over the stair, pillars round the monument and a bridge over a chasm. Built from real stone (quarried from outcrops), blocks or timber by real builders, a stretch and a storey at a time; walls and fortifications can go up on a wall top or beside something to build against, and builders may work from the level below. `k` and the director name them.
- **Performance overlay and the overseer's doings** (this commit): `F3` shows frames per second and where the time goes (simulation by part, drawing, path searches, the slowest tick); the announcements split in two while the overseer plays, its notes on the right. Found with it: a dig that uncovered a tile of open air flooded through the whole known sky (up to 60,000 tiles) to reveal it, a 70–300 ms freeze many times a year while a fort dug in near the surface; the flood now stops at what's already known. Pathfinding keeps what it has reached in flat arrays reused between searches instead of a hash map (about 14% off a batch).
- **The local map follows the world map** (16ad7d2; wildlife this commit): ground, plants and rock by biome (dunes and cacti, red clay, bare peaks, lasting snow, conifers, acacias, palms, reeds, evil-tainted growth); the neighbouring world cells reach onto the map with their biomes and heights (coasts, rising mountains, blended borders, carried on when the map grows); wildlife by biome (camels, scorpions, vipers, elk, ibex, antelope, lions, jaguars, alligators, polar bears) and walking husks on evil land.
- **Big moments, side panes, game over** (7002196): dramatic and rare events pause the game, turn the camera to them and tell them in a card (the overseer's cards go by on their own); status, brain, reports, chronicle and legends open as a side pane beside a live map (`\` for full screen); when the last dwarf dies, a game-over page tells the fortress's story. S01 (cause reports, `--story`, `--out`, `--compare`; 2e3a9c5) and the fight analysis (arena, blow counts, siege replay; c7b07f9) came before.
- **Planned fortresses and burials** (8c1e195): five layouts drawn up as halls first (great hall, mirrored wings, pillared atrium, grid of lanes, hub and spokes) with rooms opening off them through doors and rows of one-dwarf bedrooms; each overseer style prefers its own; the dead now get a tomb and coffins; hostiles with nobody to chase prowl (cave beasts in the caves, the rest in the open) instead of standing at the stairs; a warren tunnel bending into the gatehouse no longer hangs the game.
- **An atlas of a world map, smoother level changes, mouse panning** (ecfc2a1): one character a cell; ridged mountain ranges, rivers and lakes, volcanoes, both poles cold; five new biomes (jungle, taiga, savanna, badlands, glacier); bigger worlds with more peoples and beasts; a river or volcano on the world map comes with the site; the terminal is written once a frame (level changes on big maps were hundreds of small writes); a middle or left drag pans the view alone and hides the cursor until a click or arrow key.
- **Varied sites, map sizes, growing the map, faster water** (3c04e68; mouse d0c2fb0, reports 592b6dc, slowdowns e4cb4f8): twelve site features chosen per world cell and shown before embark; free map width, height and depth; `M` grows the map on any side with new land continuing the old (the save is shifted wholesale); still water sleeps until something nearby changes (a year runs two to three times faster).
- **Overseer layouts and grand works** (this commit; brain 9e557ea): five hall temperaments, room shapes (pillared, round, cross, L), rooms only where rock surrounds them and always behind the gate, the Architect's grand works. Fixes: dwarves on a prayer or carousing break now drink, eat and sleep first (they could die of thirst beside full barrels); threats behind solid rock no longer send dwarves fleeing; a desperately thirsty or hungry dwarf stops fleeing.
- **Rot and miasma** (c785694): bodies rot; underground they give off miasma that drifts, fades and sickens dwarves; after forty days they're bones (citizens wait for burial). The overseer keeps a refuse pile outside.
- **Autoplay** (3ce70ec; step 1 4f27d18): the overseer AI plays a fortress in one of five styles; `A` hands it over, *Watch a fort* on the title screen, `--autoplay` runs many and reports ruin or glory.
- **F15 Megaprojects** (95c75ce): a great monument (`b`, `G`, thirty stone blocks) that rises as scaffolding and, finished, is dedicated to the fortress's greatest hero with a feast, a legend, a milestone and a year named for it; dwarves who pass admire it. The build menu became a two-column popup so every building fits.
- **F16 Mysteries** (5c6e81e): the Whispering Stone (a gem that whispers to its miner and leads them to the sealed hall, or into a fey mood) and the hooded stranger (a long-lost sibling or friend who joins, or a goblin spy who brings the next siege for certain); each unfolds over days and happens once.
- **F17 Milestones** (5bd6098): banners across the map for the fortress's firsts (masterwork, artifact, child, festival, hero, broken siege, dragon, vault, great fish, expedition, a deathless year, the necromancer's end) and its population and wealth marks, listed on the status screen.
- **F14 The necromancer** (d66ee5c): some sites have a necromancer nearby who, after the first year, raises the dead: a horde from the map edge and our own unburied dead where they lie (more unburied, more raids); risen dwarves fall back as corpses to bury; destroying the necromancer stills every corpse for good (Deathbane).
- **F13 Wild weather** (0263e92): thunderstorms (lightning far more often, flashes on screen), blizzards that chill dwarves caught outside, summer droughts with no rain and shallow water drying up, and blood rain over evil sites that badly upsets anyone caught in it.
- **F12 Legendary catches** (97b27e3): one named great fish per site (pike, bog eel or sea serpent) bites now and then and breaks the line, easier to land with skill and each escape; landing it is a masterwork catch, a legend and an epithet.
- **F11 Pets with personality** (1bf21d3): cats adopt dwarves and dogs are taken in, with names; they follow their dwarf, cheer them, get up to mischief in the gossip, are mourned when they die, and wait for a dwarf who died.
- **F10 Grudges, romance and duels** (d34c633): pranks, jealousy between rivals in love, insults between enemies, challenges to duels at dawn fought to a yield (half end the grudge; three wins make a dwarf Unbeaten), and a Gossip tab on the status screen.
- **F09 Expeditions** (dbfd92e): `e` on the military screen sends a squad off the map to explore a ruin, raid an enemy site or hunt a megabeast in its lair; odds from squad strength against danger; they return days later with loot, epithets and legends (a hunted beast dies in the world history), or beaten and fewer.
- **F08 Ancient ruins** (8965925): every site hides a sealed marble hall of a dead civilization deep in the rock, its walls engraved with the world's oldest legends; breaking in reveals gold, gems and an artifact, and usually wakes its warden, a unique horror; older saves get one on load.
- **F06 The year's chronicle** (3acce89): each new year pauses on a parchment page telling the year that ended (population, wealth, migrants, births, festivals, wars, kills, fires, crimes, the legends' great deeds, the dead), titled after its biggest event; `c` pages through every year.
- **F04 Heroes and epithets** (a7113b9): deeds earn epithets (Dragonslayer, Titanbane, Horrorbane, Warbreaker, Grim, Red-Handed, Inspired, Ironarm, Deepcup), only ever grander; heroes fight under their epithet, go into the legends, are admired, and are mourned by everyone when they fall.
- **F03 Festivals** (8bbc56a): the harvest feast, midwinter fire and founding day; dwarves down tools for half a day, gather at the meeting hall, tavern, dining hall or wagon, dance, drink from the stores, arm-wrestle and hold a drinking contest, and remember it fondly.
- **F02 Dragons** (65388cc): dragons from the legends breathe fire down a clear line every few seconds, scorching dwarves and creatures and setting what will burn alight, never burn themselves, show as a red `D`, and when slain leave a masterwork dragon skull and the hoard they carried (gold, gems).
- **F01 War from hostile neighbours** (66d33f6): civilizations at war with yours in the world's history lay siege as well as goblins, nearer enemies more often: human soldiers, elf warriors with wooden arms, dwarf raiders in heavy armour, each army led by a living figure of their civilization (or a new one) as general; the general's fall breaks the army and goes into the legends.
- **Title screen** (258c242): an animated backdrop, a mountain under a night sky with drifting clouds and twinkling stars, trees and falling petals, leaves or snow through a quick little year, the fortress dug into it with dwarves walking its halls, a miner at the face, a forge with its chimney smoking, a magma sea bubbling below, and eyes in the dark; the title glints, and lore lines type themselves out under the menu.
- **Seasonal trees** (6e63087): blossom and buds in spring, autumn colours and fallen leaves, bare branches in winter, evergreens (most of them in cold places).
- **Posts that depend on fit** (1e10414, 3bff6d2): appoint from a list of dwarves ranked by fit (good, fair, poor, with the qualities behind it); a good manager, bookkeeper, broker, chief medical dwarf, commander or sheriff does the job better, a poor one worse.

- **W19 Temperature and weather** (this commit; steps dddc21f, 998c340): winter ice you can walk on (and still drink through); daily rain or snow by climate, snow lying until spring; fire that catches in trees, shrubs, grass and wood, spreads across slopes, burns creatures and items, and is put out by water and rain, started by magma or lightning.
- **W18 Geology and engraving** (53f697d; step 59d06e8): smoothing and engraving designations and a Stone detailing labor, engravings showing scenes from the world's legends and adding to room value; two cavern layers (three on deep maps), each with its own spiders and troglodytes. (The magma sea came with W11.)
- **W17 Monsters** (1aaaa7b; steps bc802c7, d226620): megabeasts from the world's own history come for a rich fortress after its first year, hunt to the death, and their slaying is written into the legends; generated cave horrors crawl up once the caverns are found; skulkers slip in unseen to steal the most valuable thing lying about, and bolt when a dwarf spots them.
- **W16 Justice** (8f2cffb; steps 31d7368, efd4286): a record of crimes (tantrum assault and vandalism, the dwarf blamed for an unmet mandate) on the nobles screen; a sheriff who finds each culprit and beats them, or with a jail zone (`i`, `j`) sentences them to days inside, where they do no work; victims are glad to see justice done.
- **W15 Noble positions** (df23471; steps 68ccc36, 0cbb7eb, 31a78ae): appointed manager, bookkeeper, broker, chief medical dwarf and militia commander (nobles screen, `1`-`5`); work orders need a manager and run more often with a bookkeeper; a broker cuts the merchants' markup; treatment and training speed up with their officers; the civilization makes a dwarf baron, count, then duke as wealth and population grow, and titled nobles demand furniture of a given material in their rooms, with bigger room expectations.
- **W14 Work orders and item management** (35ccda5; steps 4d0bec8, 7b7dac9, 4bac3d8): a work orders screen (`j`) with "keep N in stock" orders that queue jobs at whichever workshop can do them; forbidding items (`f`/`F` in look mode) alongside dumping; stockpile settings (`s` on a stockpile) refusing kinds and materials; workshop profiles (`w` on a workshop) limiting who may work there.
- **W13 Farming depth** (2e12f5a; steps 9bf1b98, 0b9c76f, 32d0e1a): five crops with their own seasons and places (stoutcap anywhere; hollowgrain and threadreed underground; sunberries and goldoat on the surface), farm plots that remember their crop and farms that can be set to one; seeds back from eating, brewing, cooking, spinning and milling; gathering areas over wild shrubs; a Fishing labor that fishes deep water; a millstone grinding grain into flour that bakes into more meals.
- **W12 Workshops and clothing** (df5adfc; steps 8338b8b, c1fc029, db9adb8): the butcher leaves hides and tallow; a tanner's shop and leather works (leather armor, caps, shoes, crafts); a loom and clothier's shop (cloth, shirts, trousers); dwarves dress in shirt, trousers and shoes that wear out over a year, are embarrassed by rags and change into fresh clothes from stock; ash from the wood furnace and tallow make soap at the soap maker's, which lets doctors clear infection outright. Three new labors: Leatherworking, Weaving and tailoring, Soapmaking.
- **W11 Fluids and magma** (4e81b17; steps 003591a, 3212e8c, a6d337d): wells over a channel to water (clean drinking water; doctors carry buckets to patients in bed); lever-worked floodgates, with doors and raised bridges also holding water back; a magma sea above the bedrock that flows, burns, kills and sets into obsidian against water, heating fuel-free magma smelters and forges; aquifers by site (shown at embark), light ones dampening dug floors and heavy ones flooding anything dug beside them unless walled off with constructions.
- **W10 Death, family and aging** (bb1c008): coffins (carpenter or mason) and a tomb zone; haulers bury citizens' corpses, tombs first, comforting their friends; dead left unburied for two weeks may rise as ghosts that drift about and frighten the living until buried or remembered on a memorial slab (mason). Dwarves have an age and sex, die of old age past 130, marry close friends, and have children (mother's family name, no work, come of age at twelve). Grief is heavier for spouses, parents and children. Shown on the dwarf, look and status screens.
- **W09 Personality needs** (a739cdc): seven needs (socialize, drink, craft, fight, learn, pray, make merry), each 1–3 strong by personality; they fade and are met by chatting, drinking, finishing workshop jobs, training and fighting, learning skills, and leisure in the new temple (`i`, `t`) and tavern (`i`, `v`) zones; dwarves take breaks to pray or carouse when those needs go unmet. Focus (70–120%) follows needs and scales work speed and craft quality; shown on the dwarf screen and summed up on the status screen.
- **W08 Animals** (b543de8): dogs, cats and goats (a pair of dogs, a pair of goats and a cat on embark); tame animals never turn hostile, never leave, use doors; pasture zones keep assigned animals in (new pastures take in loose goats); breeding with pregnancies, litters and young that grow up in a season (herds cap at 24 per species); war dogs follow their soldier and attack nearby hostiles; caged animals can be tamed and kept animals slaughtered by a butcher.
- **W05 Health care** (this commit): Medicine labor (doctors) and a hospital zone whose beds the wounded use first; doctors suture bleeding (thread), clean dirty wounds, splint mangled limbs (faster mending, less limping) and fit crutches for lost legs; uncleaned cuts can fester into infections that stop healing and slowly do harm. Thread is spun from stoutcap fiber until W12 brings cloth.
- **W04 Squads and the military screen** (this commit): named squads with members, a station, orders (train, hold station, kill a chosen creature) and a uniform (weapon type, armor on/off); a barracks zone where soldiers train and spar for faster learning; wooden crossbows and bolts from the carpenter, marksdwarves shooting up to 10 tiles with a clear line of fire (through fortifications), archery targets for practice.
- **W06 Mechanisms, levers and traps** (this commit): Mechanics labor and Mechanic's workshop (mechanisms from stone); wooden cages at the carpenter; buildings that take several parts; cage traps (catch a creature, then need a new cage), weapon traps (strike up to three times, re-arm themselves), stone-fall traps (one heavy hit, then need a new boulder); levers linked to doors and drawbridges (linking uses a mechanism), pulled by any civilian; a raised drawbridge blocks everyone and can't be battered down.
- **W07 Burrows and traffic** (this commit): `w` paints up to eight burrows and picks who must stay in each (members only path and work inside); one burrow can be the civilian alert burrow, and `m` then `c` sends every non-soldier into it (status bar shows the alert); `o` paints high, normal, low or restricted traffic, which changes dwarves' path costs.
- **W23 Embark and map size** (this commit): `s` on the world map picks Small 64×48×32, Medium 96×64×32 (unchanged) or Large 144×96×48 with a second, deeper cavern layer; the footprint is outlined on the world map. Saves are run-length encoded (a medium save is well under 4 MB, was ~25 MB; old saves still load). A large map simulates a year in ~12 s.
- **W22 Larger world map** (this commit): small 64×32, medium 128×64 or large 256×128 worlds (civilizations and megabeasts scale with the land; a large world generates in under 0.1 s); the world map scrolls with the cursor and shows a minimap with the visible area outlined; the generation screen shrinks the world to fit; seas, lakes, mountain ranges, forests, plains, deserts, tundra and swamps get unique names, shown in the site panel, a Regions legends tab and the founding legend.
- **W21 World generation screen** (this commit): New world opens a settings screen (preset, seed, history length, civilizations, savagery, minerals), then the world is drawn row by row and history plays out year by year with running totals and the latest events (Space hurries); accept, regenerate or go back to the settings. Generation is resumable and gives the same world stepped or in one go.
- **Fixes from a played fort** (0e62fd1): hauling search 22× cheaper, one construction warning per kind, stress floor, cooking leaves spawn.
- **W24 Title and loading screens** (this commit): title menu (New world, Continue, Load game, Quit); world generation, embarking, saving and loading run on a worker thread behind a loading screen with stage, progress bar and tips; save slots per fortress in `saves/` with a load menu (delete with confirmation); autosave each season; quitting offers save-and-quit and save-and-return-to-title.
- **Diagonal movement** (7571ac3): diagonal steps slip between two walls, as in DF (the tile stepped to must still be open).
- **W01 Constructions** (this commit): `b` then `C` plans walls, floors, stairs, ramps and fortifications in a box, built from rough stone, blocks or logs (`m` picks). `x` takes a construction down and returns its material; `c` cancels plans. Floors bridge open space; fortifications stop walkers but let water through; built walls can't be mined; a roof makes a surface room count.
- **W03 Rooms made from zones** (this commit): bedroom and office zones with owners (assigned in look mode with `o`), dormitories with shared beds, dining halls whose seats are taken first, a garbage dump with `d`/`D` to mark items. Room zones work above ground and are valued over the zone itself.
- **W02 Labor screen and work details** (this commit): `y` opens a grid of every dwarf against every labor with skill levels; `a` toggles a whole column; named work details (6 defaults, save your own, apply with 1-9); specialists never haul; mining and woodcutting are exclusive.
- **Meeting areas, look mode, status screen** (dbae659): `i` zones with meeting areas, so idle dwarves gather and chat there; `k` look at anything under the cursor; `z` status screen with overview, stocks, health and dwarves.
- **World and history** (428a1fb): world map, civilizations, 200 years of history, legends viewer, embark choice, caravans and trade.
- **Psychology** (2ba1bbb): personalities, friendships, rooms, nobles, mandates, strange moods, insanity.
- **Industry** (f69a174): seeds, kitchen, butcher, smelting, steel, forge, crafts, barrels and bins, item quality.
- **Combat** (100e73f): wildlife, caverns, sieges, body-part wounds, military drafting, doors.
- **Strange powers** (B14, branch feat/magic): gods with spheres, taboos and myths worshipped by civs, patrons and dedicated temples; a working power on every artifact; taboos, curses and atonement; omens that come true; a resident wizard with a laboratory, materials, an assistant, spells and accidents. Shown in the look panel, the status screen, the chronicle and the legends' Gods tab.
