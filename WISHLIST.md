# Wishlist

What Dwarf Fortress has that DwarfClone doesn't yet, ranked by how much it changes play.
Sourced from the [DF wiki](https://dwarffortresswiki.org/) and checked against our code.

## How to use this

- **Pick work** from "Up next", top first. Each item is split into steps that can ship on
  their own; do one step per commit where possible.
- **Mark progress** in the step list: `[ ]` todo, `[~]` in progress, `[x]` done (add the
  commit hash). When every step is done, move the item to "Done" with a one-line summary.
- **Add ideas** to "Inbox" with a line on why. Promote them into the ranked list when
  they're understood well enough to have steps.
- **"Done when"** is the test: each item should leave behind an automated test that proves it.
- Sizes: **S** one session, **M** a few sessions, **L** a big system.

## Up next

Suggested order: W02, W03 (cheap and visible) → W01 → W07, W06 (siege defense) → W04, W05.

### Tier 1: basics you'll miss in the first hour

#### W01 Constructions (L)
- **DF:** buildable walls, floors, stairs, ramps, fortifications, bridges, roads, grates, bars, windows, supports; remove construction.
- **Us:** digging only. Rock can be removed, never placed.
- **Plan:** a construct designation that consumes a boulder, block or log and sets the tile's `kind` and `mat` (`world.rs` `Tile`). Reuse the hauling and claiming flow of `find_construct` and `Task::Construct` in `game.rs`, with a new task that targets a tile instead of a building. Mark constructed tiles so they can be removed and drop their material back.
- **Steps:**
  - [ ] Walls and floors from boulders, blocks or logs; build menu entry with a box selection
  - [ ] Constructed stairs (up, down, up/down) and ramps
  - [ ] Remove construction (gives the material back)
  - [ ] Fortifications (soldiers can see through, creatures can't pass)
  - [ ] Roof tiles over surface rooms count toward room detection (`psyche.rs` `room_at`)
- **Done when:** a test walls off a corridor and a goblin can no longer path through it; removing the wall restores the path.

#### W02 Labor screen and work details (S)
- **DF:** work details (Miners, Haulers, Planters…) set to "only selected", "everybody" or "nobody"; specialised dwarves; mining, woodcutting and hunting exclusive.
- **Us:** 13 labor toggles, edited one dwarf at a time on the unit screen.
- **Plan:** a new `Mode::Labors` full-screen grid (dwarves are rows, labors are columns), moving the cursor and toggling with Enter. Work details are named presets of labors, applied to a dwarf.
- **Steps:**
  - [ ] Grid screen with row and column cursor; toggle a cell; toggle a whole column
  - [ ] Work details: named labor sets that can be applied to a dwarf, with default sets
  - [ ] "Specialised" flag: the dwarf only does their own labors, never hauling
  - [ ] Exclusive tool labors: one of mining, woodcutting or hunting per dwarf
- **Done when:** a test toggles a column and every dwarf's labor changes; UI render test shows the grid.

#### W03 Rooms made from zones (S–M)
- **DF:** zones for bedroom, dormitory, dining hall, office, tomb, barracks, hospital, tavern, temple, library, garbage dump, water source, pit/pond, pen/pasture, dungeon.
- **Us:** rooms are detected automatically from enclosed digging, beds are handed out automatically, and there's a meeting area.
- **Plan:** more `ZoneKind` variants in `game.rs` and more `ZoneTool` entries in `app.rs`. Assigning a bedroom to a dwarf replaces automatic bed assignment when used.
- **Steps:**
  - [ ] Bedroom zone, assigned to one dwarf (from look mode or the zone screen)
  - [ ] Dormitory zone: shared beds; mild bad thought
  - [ ] Dining hall zone: dwarves prefer tables inside it
  - [ ] Garbage dump zone, plus a dump command on items (ties to W14)
  - [ ] Office zone (needed by later noble positions, W15)
- **Done when:** a test assigns a bedroom and the owner sleeps there, not in a nearer free bed.

#### W04 Squads and the military screen (M–L)
- **DF:** squads with captains and a militia commander, uniforms, monthly schedules, station, patrol, burrow and kill orders, barracks, sparring, marksdwarves with crossbows and bolts.
- **Us:** individual drafting, one global station point, a global engage switch, hunting.
- **Plan:** a `Squad` struct (members, station, order, uniform) on `Game`; `combat.rs` `choose_military` reads the dwarf's squad instead of global state.
- **Steps:**
  - [ ] Squads: create, name, add and remove members; per-squad station point
  - [ ] Kill order: pick a target with the look cursor
  - [ ] Uniforms: weapon type and armor per squad
  - [ ] Barracks zone: soldiers train and spar there
  - [ ] Crossbows, bolts and an archery target (first ranged combat)
- **Done when:** a test sends two squads to different stations and both arrive; a kill order kills the named target.

#### W05 Health care (M)
- **DF:** a hospital zone and a chief medical dwarf, diagnosis before treatment, surgery, sutures, bone setting, cleaning and dressing wounds, splints, casts, crutches; thread, cloth and soap; infection from dirty water.
- **Us:** wounds, bleeding and severed limbs (`body.rs`); faster healing in bed.
- **Plan:** a hospital zone (W03) and a doctor labor. Patients are carried or walk there; treatments are tasks on the patient.
- **Steps:**
  - [ ] Hospital zone and doctor labor; wounded dwarves go there instead of any bed
  - [ ] Suturing stops bleeding (needs thread; a stand-in item until W12)
  - [ ] Splints stop limping sooner; crutches for lost legs
  - [ ] Infection: untreated wounds can worsen; cleaning with water prevents it
- **Done when:** a test wounds a dwarf, a doctor sutures them, and they stop bleeding sooner than an untreated one.

### Tier 2: what makes a fortress a fortress

#### W06 Mechanisms, levers and traps (M)
- **DF:** a mechanic's workshop making mechanisms; levers and pressure plates linked to doors, bridges and floodgates; cage, weapon, stone-fall and spike traps.
- **Us:** doors you can lock.
- **Steps:**
  - [ ] Mechanic labor and workshop; mechanisms from stone
  - [ ] Cage trap (catches a creature; the cage becomes an item)
  - [ ] Weapon trap and stone-fall trap
  - [ ] Lever linked to a door; then a drawbridge (needs W01)
- **Done when:** a test sends a goblin down a corridor with a weapon trap and the goblin is hurt or killed.

#### W07 Burrows and traffic (S–M)
- **DF:** burrows that restrict where dwarves may go, a civilian alert that sends everyone to a burrow, traffic designations (high, low, restricted).
- **Us:** civilians run from threats they can see.
- **Plan:** burrows are named tile sets; a dwarf's allowed area filters job search and pathing. Traffic is a per-tile cost multiplier in `path.rs`.
- **Steps:**
  - [ ] Burrows: create, paint tiles, assign dwarves; jobs outside are skipped
  - [ ] Civilian alert: one key sends every civilian to the alert burrow
  - [ ] Traffic designations: high, normal, low, restricted path costs
- **Done when:** a test raises the alert and every civilian ends inside the burrow.

#### W08 Animals (M)
- **DF:** pets, livestock, breeding, pastures, milking, shearing, war and hunting dogs, taming, cages, embark animals.
- **Us:** wild animals only, which can be hunted and butchered.
- **Steps:**
  - [ ] Tame species (dog, cat, goat…), brought on embark
  - [ ] Pen/pasture zone; animals stay in it
  - [ ] Breeding and young
  - [ ] War dogs assigned to soldiers; taming caught animals (needs W06 cage traps)
- **Done when:** a test leaves a breeding pair in a pasture for a season and the count goes up.

#### W09 Personality needs (M)
- **DF:** about 30 needs (pray, drink, socialise, craft, fight, learn, wander…); unmet needs lower focus, which changes skill by up to ±50%.
- **Us:** thirst, hunger and sleep; 5 traits; thoughts; friendships and grudges (`psyche.rs`).
- **Steps:**
  - [ ] Needs framework: per-dwarf need levels from personality; decay; satisfied by activities
  - [ ] First needs: socialise, drink alcohol, craft, fight, learn
  - [ ] Focus affects work speed and item quality
  - [ ] Temple and tavern zones meet pray and party needs
- **Done when:** a test keeps a dwarf from crafting and their focus drops; crafting restores it.

#### W10 Death, family and aging (M)
- **DF:** burial, coffins, tombs, memorial slabs, ghosts from unburied dead, old age, marriage, children.
- **Us:** corpses and grief thoughts.
- **Steps:**
  - [ ] Coffins and a tomb zone; burying a corpse
  - [ ] Ghosts: unburied citizens return and scare dwarves until laid to rest (slab)
  - [ ] Age and death of old age
  - [ ] Couples from close friendships; children who grow up
- **Done when:** a test leaves a corpse unburied and a ghost appears; burying it prevents that.

### Tier 3: depth

#### W11 Fluid machines and magma (L)
- **DF:** wells, pumps, floodgates, water wheels, magma forges and smelters, aquifers.
- **Us:** water that falls and levels out (`game.rs` `fluids`).
- **Steps:** [ ] well (clean water, ties to W05) · [ ] floodgate on a lever (needs W06) · [ ] magma sea and magma workshops · [ ] aquifer layers

#### W12 More workshops and clothing (M)
- **DF:** leather, loom, clothier, glass, kiln, jeweler, soap and siege workshops; clothes wear out.
- **Us:** 9 workshops, 27 recipes.
- **Steps:** [ ] tanner and leatherworker · [ ] farm thread, loom, clothier · [ ] clothes that wear out, with a bad thought for rags · [ ] soap (ties to W05)

#### W13 Farming depth (S–M)
- **DF:** many crops by season and biome, surface farming, mills, plant gathering, fishing, fruit trees.
- **Us:** one crop year-round, with seeds.
- **Steps:** [ ] 4–6 crops by season and surface or underground · [ ] plant gathering zone · [ ] fishing · [ ] mill and flour

#### W14 Work orders and item management (M)
- **DF:** global work orders with conditions; workshop profiles; forbid, dump, melt; detailed stockpile settings; minecarts.
- **Us:** per-workshop order queues with repeat.
- **Steps:** [ ] global orders with "keep N in stock" conditions · [ ] forbid and dump on items (look mode) · [ ] stockpile material filters · [ ] workshop profiles (who may work there)

#### W15 Noble positions (M)
- **DF:** broker, bookkeeper, manager, sheriff, militia commander, chief medical dwarf; titles rising with wealth up to monarch.
- **Us:** expedition leader, then mayor.
- **Steps:** [ ] appoint positions screen · [ ] manager unlocks W14 global orders · [ ] broker improves trade prices · [ ] titles by wealth, with demands

#### W16 Justice (S–M)
- **DF:** sheriff, crimes, interrogation, beatings, jail, hammerer.
- **Us:** mandates that only upset the mayor.
- **Steps:** [ ] crimes from broken mandates and tantrum vandalism · [ ] sheriff and punishments · [ ] jail zone

#### W17 Monsters (M)
- **DF:** megabeasts, titans, forgotten beasts, werebeasts, vampires, undead, thieves and kidnappers.
- **Us:** 5 wild species plus goblin sieges (`creature.rs`, `combat.rs`).
- **Steps:** [ ] megabeasts from our own world history attack the fort (`overworld.rs` figures) · [ ] forgotten beasts in the caverns · [ ] thieves who steal goods

#### W18 Geology and engraving (M)
- **DF:** smoothing and engraving walls, multiple cavern layers, magma sea, prospecting.
- **Us:** soil, two rock layers, ore veins, gems, one cavern layer.
- **Steps:** [ ] smooth designation · [ ] engrave (raises room value, depicts world history) · [ ] 2–3 cavern layers

#### W19 Temperature and weather (L)
- **DF:** temperature, freezing, rain and snow, fire, burning.
- **Us:** a calendar with seasons only.
- **Steps:** [ ] surface water freezes in winter · [ ] rain and snow · [ ] fire spreading through wood

#### W20 Attributes and skill decay (S–M)
- **DF:** body attributes (strength, agility…), skill decay, social skills, books, teaching.
- **Us:** 13 skills with experience, plus fighting experience (`dwarf.rs`).
- **Steps:** [ ] attributes that affect combat and work speed · [ ] skills fade without use · [ ] teaching and demonstrations

## Beyond DF: ideas from Tarn's dev page

From [Bay 12's development page](https://www.bay12games.com/dwarves/dev.html). These are DF's
own plans; some have shipped since the page was written (taverns, temples and libraries;
wheelbarrows and minecarts), many haven't. Only the ideas that fit fortress mode or our world
map are kept here. Several build on the world history we already generate (`overworld.rs`),
which is where we can go beyond the original. Promote an idea into the ranked list when it's
next. The size is our guess.

| ID | Idea | What it would add | Builds on | Size |
|---|---|---|---|---|
| B01 | Scouts and outside armies | Armies visible on the world map; scouts warn of a siege days ahead; sieges can be delayed or turned back | world map, sieges | M |
| B02 | Diplomacy | Messengers to and from other sites; siege negotiation (pay tribute or fight); petitions from visitors | civilizations, W15 | M |
| B03 | Army missions | Send squads off-map: raids, demanding tribute, freeing prisoners, taking over a site (which makes your fort a barony) | W04, world map | L |
| B04 | Villains and plots | Thieves, assassins and infiltrators run by world figures; interrogation uncovers plots; rumors screen | W16, history figures | L |
| B05 | Artifact politics | Invaders demand or steal artifacts; rumors of where artifacts are; squads sent to recover them | strange moods, B03 | M |
| B06 | Siege AI | Attackers dig, bridge or climb around defenses and learn from failed assaults | W01, W06 | M |
| B07 | Better traps | Rolling stones, multi-tile weapon traps, cage traps that can (with effort) catch megabeasts | W06 | S–M |
| B08 | Taverns, temples, libraries | Performances, guests renting rooms, prayer, books written by dwarves | W03, W09 | M |
| B09 | Guilds and sects | Skilled dwarves form guilds and religious groups that demand halls and temples, take sides in grudges, and grant perks | W09, W15 | M |
| B10 | Starting scenarios | Frontier settlement, mining company, military citadel, prison colony, roadside inn; each changes migrants, caravans and goals | embark | M |
| B11 | Hauling | Carry several items at once; wheelbarrows; minecarts on tracks | W01 | M |
| B12 | Living farms | Soil moisture and irrigation, nutrients and fertiliser, weeds and pests, planting trees | W13, W11 | M |
| B13 | Lighting | Dark caverns; torches, candles and lamps; light affecting moods and ambushes | W18 | M |
| B14 | Myths and magic | Generated creation myths, deities with powers, wizards and covens, magical invasions | worldgen | XL |

## Inbox

Ideas not ranked yet. Add a line on why each matters.

- Announcements history screen (DF's report log): find out what happened while you weren't looking.
- Mini-map and saved camera spots: get around a 96×64 map faster.
- Adventure mode: explore the generated world. Out of scope until fortress mode is deep.
- Hunting labor and hunters with crossbows (after W04's crossbows).

## Done

- **Meeting areas, look mode, status screen** (dbae659): `i` zones with meeting areas, so idle dwarves gather and chat there; `k` look at anything under the cursor; `z` status screen with overview, stocks, health and dwarves.
- **World and history** (428a1fb): world map, civilizations, 200 years of history, legends viewer, embark choice, caravans and trade.
- **Psychology** (2ba1bbb): personalities, friendships, rooms, nobles, mandates, strange moods, insanity.
- **Industry** (f69a174): seeds, kitchen, butcher, smelting, steel, forge, crafts, barrels and bins, item quality.
- **Combat** (100e73f): wildlife, caverns, sieges, body-part wounds, military drafting, doors.
