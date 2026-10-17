# Wishlist

What DwarfClone should do next, and why. Ideas come from playing and watching forts, from
Dwarf Fortress itself (checked against the [DF wiki](https://dwarffortresswiki.org/)) and from
Tarn's [development page](https://www.bay12games.com/dwarves/dev.html). We favour fun and
emergent drama over intricate systems: every feature should make a moment you'd tell someone
about. Losing is fun, but a fortress has to live long enough to have a story.

## How to use this

- **Pick work** from the roadmap, top milestone first, items in order. Each item is split into
  steps that ship on their own: one step per commit where possible.
- **Mark progress** in the step list: `[ ]` todo, `[~]` in progress, `[x]` done (add the commit
  hash). When every step is done, move the item to "Done" with a one-line summary.
- **Done when** is the test: each item leaves an automated test behind that proves it. Items
  about balance are measured with the autoplay batch (`--autoplay --runs N --years Y --seed S`).
- **Add ideas** to "Inbox" with a line on why. Promote them into a milestone once they have steps.
- Sizes: **S** one session, **M** a few sessions, **L** a big system.

## Where we stand (October 2026)

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
- **Steps:** [ ] dwarves who are bleeding stop work and press their wounds (slower bleeding),
  and friends bandage them with cloth or thread where they lie · [ ] the unconscious and the
  crippled are carried to a hospital bed by a doctor or any hauler, even during an alert, once
  the threat is gone · [ ] the overseer always has a doctor and a hospital before the first siege
  · [ ] bleeding is tuned: a cut limb kills in minutes, a gash in hours
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
  they remember: a broken siege brings a bigger one, and they say so (this commit) · [ ]
  captives
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
  [x] soldiers hold the gatehouse (stationed by the lever) · [ ] after a siege
  it learns: more traps where the enemy came in, a second squad, walls · [x] cavern breaches
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
- **Done when:** a scripted siege against a fort with shelter, bridge and squad kills fewer than
  half the goblins' worth of dwarves; the batch meets the milestone goal.

#### S06 Expeditions worth the risk (S)
- **Why:** 43 of 451 deaths (10%) were dwarves who fell on expeditions, most of them in the
  first year, when a fort can least spare them.
- **Steps:** [ ] the overseer sends expeditions only with a strong, armed squad and spare
  dwarves at home · [ ] odds are shown before sending, and a squad that's losing comes home early
  with its wounded
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
  · [ ] wells and cisterns inside the fort
· [x] a dwarf in a fey mood
  gets the workshop and materials they demand: the overseer treats those as troubles of its own,
  and urgent ones (this commit)
- **So far:** thirst and starvation deaths 93 (s03) → 66 (s05b) → 51 with the troubles
  (batches/troubles.json); no sane or melancholic dwarf died of thirst there, all 38 thirst
  deaths were dwarves driven mad by a failed fey mood, which this commit goes after.

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
  fortress short of stone…") (7579248) · [ ] pick the traits by hand · [ ] more of its
  notes in its voice
- **Done when:** two overseers with the same style but different traits build visibly different
  forts from the same seed.

#### O02 Plans that fit the land (M)
- **Why:** every plan runs east from the gate; on a narrow site most of it is wasted.
- **Steps:** [ ] a plan can face east, west, north or south, and the overseer picks the side with
  the most solid rock · [ ] a level's plan stretches to the space available, and deeper levels
  get their own layout (workshops on one, bedrooms on another, like DF forts) · [ ] `p` shows the
  plan as a ghost overlay on the map: halls and rooms still to come
- **Done when:** on a site with rock only to the west, a planned fort still fills its plan.

#### O03 Halls that look lived in (M)
- **Why:** finished forts in DF are smoothed, engraved, furnished and named.
- **Steps:** [ ] the main halls are smoothed, then engraved, as the fort grows rich · [ ] bedrooms
  get a cabinet and a chest; nobles get suites with an office and a dining room · [ ] the atrium
  and the hub get statues and the monument · [ ] halls are named after heroes and events
  ("the Hall of the Unbeaten") and the names show in the look panel and the chronicle
- **Done when:** a three-year fort has smoothed main halls, furnished bedrooms and named halls.

#### O04 Defences in the plan (M)
- **Why:** DF forts are famous for moats, trap halls and walled courtyards.
- **Steps:** [ ] the gatehouse grows with the fort: a trap corridor, then a moat under a
  drawbridge, then towers with fortifications · [ ] Warlord and cautious overseers build them
  sooner · [ ] a siege that breaks on the defences goes into the legends
- **Done when:** a Warlord fort has a moat and a trap corridor before its third siege.

### Milestone 3: the world comes to you (v0.5)

Goal: the world map is alive. What happens out there reaches the fort, and the fort reaches back.

#### B01 Scouts and armies on the world map (M)
- **Why:** sieges appear from nowhere. Seeing an army march across the map is a story.
- **Steps:** [ ] armies and caravans move between sites on the world map, drawn on it · [ ] a
  scout warns days before a siege, with its size and leader · [ ] roads and tracks between
  sites drawn on the world map
- **Done when:** a siege is announced days ahead and the army can be seen approaching.

#### B02 Diplomacy (M)
- **Why:** neighbours should be people, not just a siege timer.
- **Steps:** [ ] envoys arrive with demands (tribute, an artifact, a prisoner) or offers
  (peace, trade) · [ ] paying or refusing changes the next siege · [ ] the overseer decides by its
  traits
- **Done when:** paying tribute delays a siege and refusing brings it sooner.

#### F07 Monster hunters and performers (S)
- **Why:** finishes the wandering heroes (bards and warriors are done).
- **Steps:** [ ] monster hunters come for a beast in your caverns and may die trying · [ ]
  performers dance and tell stories in the tavern, and the stories come from the legends
- **Done when:** a hunter arrives when a cave horror lives and goes after it.

#### W25 Getting around big maps (S)
- **Why:** maps grow to 320×256 now; scrolling there is slow.
- **Steps:** [ ] a fortress mini-map (toggle) with the view outlined, clickable · [ ] saved
  camera spots (Ctrl+1-9 to save, 1-9 to jump) · [ ] typed world names and favourite seeds
- **Done when:** clicking the mini-map jumps the view there.

### Milestone 4: deep fortress (v0.6)

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
- **Done when:** a creature in an unlit cavern can't be seen until a dwarf is close.

#### W26 Water and magma works (M)
- **Why:** pumps, water wheels and magma channels are DF's great engineering projects.
- **Steps:** [ ] screw pumps lift water or magma a level · [ ] aquifer tools: pumping a shaft dry,
  and a warning before embarking on a heavy aquifer · [ ] magma creatures in the sea
- **Done when:** a pump stack drains a flooded shaft.

### Later

- **W20 Attributes and skill decay (S–M):** body attributes (strength, agility…) that affect
  combat and work speed; skills fade without use; teaching and demonstrations.
- **B03-B14** from Tarn's page: army missions, villains and plots, artifact politics, siege AI that
  digs and bridges, better traps, guilds, starting scenarios, multi-item hauling and minecarts,
  living farms, myths and magic.

| ID | Idea | What it would add | Builds on | Size |
|---|---|---|---|---|
| B03 | Army missions | Send squads off-map: raids, demanding tribute, freeing prisoners, taking over a site (which makes your fort a barony) | F09 expeditions, world map | L |
| B04 | Villains and plots | Thieves, assassins and infiltrators run by world figures; interrogation uncovers plots; rumors screen | W16, history figures | L |
| B05 | Artifact politics | Invaders demand or steal artifacts; rumors of where artifacts are; squads sent to recover them | strange moods, B03 | M |
| B06 | Siege AI | Attackers dig, bridge or climb around defenses and learn from failed assaults | S03, O04 | M |
| B07 | Better traps | Rolling stones, multi-tile weapon traps, cage traps that can (with effort) catch megabeasts | W06 | S–M |
| B08 | Taverns, temples, libraries | Performances, guests renting rooms, prayer, books written by dwarves | W03, W09 | M |
| B09 | Guilds and sects | Skilled dwarves form guilds and religious groups that demand halls and temples, take sides in grudges, and grant perks | W09, W15 | M |
| B10 | Starting scenarios | Frontier settlement, mining company, military citadel, prison colony, roadside inn; each changes migrants, caravans and goals | embark | M |
| B11 | Hauling | Carry several items at once; wheelbarrows; minecarts on tracks | W01 | M |
| B12 | Living farms | Soil moisture and irrigation, nutrients and fertiliser, weeds and pests, planting trees | W13, W11 | M |
| B14 | Myths and magic | Generated creation myths, deities with powers, wizards and covens, magical invasions | worldgen | XL |

## Inbox

Ideas not ranked yet, with a line on why.

- Workshop orders that wait on missing materials still announce often ("Cut a gem needs rough
  gem"); fold into W14's global orders.
- Overlapping zones: a tile holds one zone, so a dining hall can't also be the meeting area as it
  often is in DF.
- Roads, grates, bars, windows and supports: the rest of DF's constructions.
- Drawbridges over open space (moats, see O04), wider than one tile, flinging what's on them;
  pressure plates.
- Linking a lever uses a mechanism instantly; DF has a mechanic walk over and install it.
- Caged animals could be tamed (W08) and caged goblins interrogated (B04).
- Squad schedules (DF's monthly alerts) and patrol routes; metal crossbows and bolts at the forge.
- Diagnosis and surgery (DF's fuller medicine, after S02).
- Grazing and animal hunger, milking and shearing, animal trainers, egg-laying fowl.
- More needs (DF has ~30): wander, see family, admire art, eat good food, be creative; a library
  for "learn" with books (B08).
- Deities and prayer to a specific god; temples dedicated to one (B14).
- Violent ghosts, ghosts that move objects, and haunting that varies with how the dwarf died.
- Nursing babies, children who follow their mother, a playground or school; sibling and
  grandparent ties.
- Glass furnace, kiln, jeweler and siege workshop (ballistae, catapults): the rest of DF's
  workshops.
- Dyes and dyed clothes; clothing preferences; cloaks, gloves, hoods; children outgrowing clothes.
- Fish stocks that run out; fish cleaned at a fishery; bees, honey and mead; orchards.
- Melting metal items back into bars; forbidding by area from the designation menu.
- Stockpile links that feed a workshop; minimum skill on workshop profiles.
- A monarch arriving at the top of the titles; nobles' demands for artifacts, banishment and the
  hammerer.
- Theft with witnesses and interrogation; chains and cages in the jail; prisoners fed by others.
- Werebeasts, vampires and kidnappers.
- Prospecting and ore seen through smoothed walls; deeper caverns with stranger creatures; a
  cavern fungus forest for underground wood.
- A true temperature model (heat from magma, cold snaps killing crops, warm clothes); bucket
  brigades against fire.
- Murky puddle water should feel worse than river water.
- Adventure mode: explore the generated world. Out of scope until fortress mode is deep.

## Done

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
