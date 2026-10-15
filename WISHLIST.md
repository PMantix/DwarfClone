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

Suggested order: the rest of F05 and F07, W20, then the Beyond DF list (B01-B02 next: diplomacy and armies on the map build on F01).

### Tier 1: basics you'll miss in the first hour

### Tier 2: what makes a fortress a fortress

### Tier 3: depth

#### W20 Attributes and skill decay (S–M)
- **DF:** body attributes (strength, agility…), skill decay, social skills, books, teaching.
- **Us:** 13 skills with experience, plus fighting experience (`dwarf.rs`).
- **Steps:** [ ] attributes that affect combat and work speed · [ ] skills fade without use · [ ] teaching and demonstrations

## Fun first: ideas picked for drama and delight

Our own list of features that make stories, chosen because they're exciting to watch or play
rather than intricate to manage. Each should create moments you'd tell someone about. Ranked
by fun for the effort; promote one into "Up next" with steps when it's next.

| ID | Idea | The moment it makes | Builds on | Size |
|---|---|---|---|---|
| F05 | Disasters (cave-ins done) | ~~Cave-ins~~ (8e392cc); still to come: earthquakes crack walls; the spring melt floods the lower halls; the magma sea surges | W11, W01 | S–M |
| F07 | Bards and wandering heroes (bards, warriors done) | ~~Bards, wandering warriors who join~~ (1ec4734); still to come: monster hunters who come for the beast in your caverns, performers who dance or tell stories | B08 taverns, legends | S |

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
- A monarch arriving at the top of the titles; nobles' demands for specific artifacts, banishment and the hammerer; noble offices with a minimum value.
- Theft and other crimes with witnesses and interrogation; the hammerer; chains and cages in the jail; prisoners fed by others.
- Werebeasts, vampires and the undead; dragons breathing fire; kidnappers; thieves who are caught and jailed (W16).
- Prospecting and ore visible through smoothed walls; deeper caverns holding stranger creatures; a cavern-floor fungus forest for underground wood.
- A true temperature model (heat from magma and fire, cold snaps killing crops, dwarves needing warm clothes); bucket brigades against fire; dragons breathing it.
- Murky or stagnant water drunk from puddles should feel worse than river water; doctors could clean wounds with well water and soap.
- Corpses that rot (miasma) and refuse that needs a stockpile far from the halls.
- Mini-map and saved camera spots: get around a 96×64 map faster.
- Adventure mode: explore the generated world. Out of scope until fortress mode is deep.
- Hunting labor and hunters with crossbows (after W04's crossbows).

## Done

- **Rot and miasma** (this commit): bodies rot; underground they give off miasma that drifts, fades and sickens dwarves; after forty days they're bones (citizens wait for burial). The overseer keeps a refuse pile outside.
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
