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

Suggested order: finish W15 (broker next), then onward in tier order.

### Tier 1: basics you'll miss in the first hour

### Tier 2: what makes a fortress a fortress

### Tier 3: depth

#### W15 Noble positions (M)
- **DF:** broker, bookkeeper, manager, sheriff, militia commander, chief medical dwarf; titles rising with wealth up to monarch.
- **Us:** expedition leader, then mayor.
- **Steps:** [x] appoint positions screen (68ccc36) · [x] manager unlocks W14 global orders (this commit) · [ ] broker improves trade prices · [ ] titles by wealth, with demands

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

- Typed world names and a "favourite seeds" list on the settings screen.
- Workshop orders that wait on missing materials still announce often ("Cut a gem needs rough gem"); fold into W14's global orders.
- Overlapping zones: a tile holds one zone, so a dining hall can't also be the meeting area as it often is in DF.
- Bridges and drawbridges (with W06 levers), roads, grates, bars, windows, supports: the rest of DF's constructions.
- A large embark's footprint spans several world cells but takes its biome from the cursor cell only; blend neighbouring biomes into the map.
- Drawbridges over open space (moats), wider than one tile, flinging what's on them; pressure plates.
- Linking a lever consumes a mechanism instantly; DF has a mechanic walk over and install it.
- Caged animals could be tamed (W08) and caged goblins interrogated (B04).
- Squad schedules (DF's monthly alerts) and patrol routes; metal crossbows and bolts at the forge.
- Diagnosis, surgery and soap (DF's fuller medicine); a chief medical dwarf (W15).
- Grazing and animal hunger, milking and shearing, animal trainers and training levels, egg-laying fowl.
- Pick a room's owner from a list instead of cycling through dwarves with `o`.
- Announcements history screen (DF's report log): find out what happened while you weren't looking.
- More needs (DF has ~30): wander, see family, admire art, eat good food, be creative; a library to meet "learn" with books (B08).
- Deities and prayer to a specific god; temples dedicated to one (B14); drinking water could meet the drink need a little.
- Violent ghosts, ghosts that move objects, and haunting that varies with how the dwarf died (DF has several kinds).
- Nursing babies, children who follow their mother, and a playground or school; sibling and grandparent ties.
- Pumps and water wheels (DF's screw pumps lift water a level; wheels make power); wells that freeze in winter.
- Magma creatures (fire imps, magma crabs) in the sea; magma moved by pumps or minecarts into traps and forges higher up.
- Aquifer tools: pumps to drain a flooded shaft, and DF's trick of freezing or caving in an aquifer; warn before embarking on a heavy one.
- Glass furnace, kiln (pottery, glazes), jeweler (gem setting) and siege workshop (ballistae, catapults): the rest of DF's workshops from W12's list.
- Dyes and dyed clothes; clothing preferences; cloaks, gloves, hoods; children outgrowing clothes.
- Fish stocks that run out with overfishing; fish that must be cleaned at a fishery; bees, honey and mead; fruit trees and orchards.
- Melting metal items back into bars at the smelter (DF's melt mark), and forbidding by area from the designation menu.
- Minecarts and tracks (DF's hauling network); stockpile links that feed a workshop; minimum skill on workshop profiles.
- Murky or stagnant water drunk from puddles should feel worse than river water; doctors could clean wounds with well water and soap.
- Corpses that rot (miasma) and refuse that needs a stockpile far from the halls.
- Mini-map and saved camera spots: get around a 96×64 map faster.
- Adventure mode: explore the generated world. Out of scope until fortress mode is deep.
- Hunting labor and hunters with crossbows (after W04's crossbows).

## Done

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
