# World, interface and the overseer

Part of the [idea catalogue](README.md): what Dwarf Fortress does (as Bay 12 describes it), what
DwarfClone has, and every distinct idea, in our own words. **Roadmap** says which WISHLIST item
an idea belongs to; ideas without one are free to pick up.

## World, history & legends (WLD)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| World generation | Long history, many peoples, settings for size, savagery, magic and realism; drawn live as it runs | Three world sizes, 13 biomes, ridged mountains, rivers, named regions, settings screen, stepwise resumable gen (`overworld.rs`) | Small: no magic/realism dials, no editors |
| Civilizations and sites | Peoples with ethics, knowledge, religions, companies; towns, hamlets, fortresses, lairs, camps | 4 races, 6 site kinds, figures with titles and kills (`overworld.rs`) | Civs have no ethics, knowledge or factions |
| History after embark | The world keeps living: births, wars, sieges, successions, banditry go on during play | History stops at embark; only sieges, caravans and expeditions touch it (`game/combat.rs`, `trade.rs`, `expeditions.rs`) | Large: frozen world |
| Wars and diplomacy | Wars follow ethics and grudges, alliances against great evils, conquered sites get one of five fates | Wars and peace among civs in worldgen only; at-war civs send sieges (F01) | Wars can't start or end in play |
| Historical figures | Figures with personalities, relationships, plots and villainy | Figures with titles, kills, lairs; generals lead real armies | Figures have no inner life |
| Legends viewer | A browser with links, filters, nicknames, regions, a creature list | `L` legends with tabs incl. Regions; fortress deeds written in | No search or filter, no links |
| Named events | Fights grouped into named battles and raids; places named after what happened | Chronicle years titled after their biggest event (`game/chronicle.rs`); siege notes (`annals.rs`) | Sieges and halls aren't named |
| Rumours and news | News travels in people's heads; liaison carries world news; villains plotted via rumours | Word of mouth from merchants and deep-road tappers sets migrant wants (`game/word.rs`) | No news of the world reaches the fort |
| Fallen and retired forts | A lost fort becomes a ruin in the world: looted, occupied, haunted, reclaimable | Game over saves the dead fort as a slot; "Look upon the ruins" (`ui.rs`) | The world never learns of it |
| Artifacts in the world | Worldgen makes artifacts that change hands by theft, gift and loot | Sealed ruin holds one artifact of legend (`game/ruins.rs`) | No circulation |
| Weather and sky | Fronts, storms, tornadoes, dust storms, thin ice, deep snow, puddles, named stars and eclipses | Rain/snow by climate, thunderstorms, blizzards, drought, blood rain, walkable ice (`game/weather.rs`, F13/W19) | No sky events, no thin ice, no wind storms |
| Map dressing | Seasonal plants, many grasses, lake plants, shores with tide pools, footprints | Seasonal trees, biome plants, coasts (`terrain.rs`) | Cosmetic gaps |
| Ages and myths | Named ages that shift with the world; a mythic age to start in; other planes | None | Missing |
| Languages | Generated languages with grammar and scripts | Generated names (`game/word.rs`) | Cosmetic |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| WLD-01 | Named episodes and battles | A siege, massacre or great hunt is grouped under a name given afterwards ("the Ambush at Copper Gate") and listed as one event in the legends and chronicle. | Partial: chronicle years named for their biggest event; SiegeNote records | S | ★★★ | O09 | Bloat#219, Bloat#270, Bloat#244 (via roadmap "Battle cries and named battles"), roadmap Legends you can search |
| WLD-02 | Places named for what happened there | A hall, gorge, ford or tunnel takes a name from the deed or death that happened in it, shown on look and in legends. | Planned: O03 (named halls) | S | ★★★ | O03 | Story:Gorge of the Righteous, Req421 (named halls) |
| WLD-03 | Sky events | Eclipses, a blood moon, conjunctions and named constellations; work stops, the superstitious fret, the pious feast. | Partial: stars on the title screen only | S | ★★★ | D11 | Bloat#346, PG161, Arc Weather, Story:Dwarven Servitude, Story:Tales Foretold: The Reign of Cenaster |
| WLD-04 | Thin ice that breaks | Ice holds only after a hard freeze; early or late in winter it cracks under weight and dumps walkers into the water. | Partial: walkable winter ice (W19) | S | ★★★ | D11 | Bloat#287, roadmap Snow, ice and rain |
| WLD-05 | The world turns each year | At year end a light pass rolls history forward (rulers die, sites change hands, beasts slain or rampaging) and the fort gets a news digest. | Missing | M | ★★★ | B17 | Core77, Arc Life Cycle, 2016, Log2000s (Living world during play, Weekly world events) |
| WLD-06 | Wars declared and ended in play | Civs go to war or make peace during the game from grudges, ethics and tribute, which changes who besieges and who trades. | Missing: wars fixed at embark | M | ★★★ | B17 | Core45-46, 2011 III.A.1, Log2000s (War from ethics, Ethics as causes of war) |
| WLD-07 | News and rumours reach the fort | Merchants, migrants and visitors bring rumours (a war, a dragon seen, a plot), some false; the overseer and dwarves can act on them. | Partial: word.rs spreads the fort's reputation outward only | M | ★★★ | B17 | Log2020s (Rumours), Log2000s (Rumours travel with people), roadmap Rumours and tips |
| WLD-08 | Prophecies | An engraving or tablet foretells an event (a beast's return, an eclipse, a fall); it may come true and the fort can try to prevent it. | Missing | M | ★★★ | D11 | Story:Tales Foretold: The Reign of Cenaster, Story:catalogue |
| WLD-09 | Legends that drift | The same event is told differently by different tellers; tales grow, fade into children's stories, and some never happened. | Missing | M | ★★★ | O09 | Story:Dog-man, Story:The Adventures of Gornon, Bloat#196, Bloat#192 |
| WLD-10 | Fallen forts enter the world | A lost fort becomes a ruin site on the world map, looted and occupied by its killers or a megabeast that moves in, and its fall is a world event. | Partial: dead fort kept as a save slot | M | ★★★ | P01 | Core70, Req266, Req61, Bloat#20, Log2000s (Megabeasts take over forts) |
| WLD-11 | Wild weather, more of it | Hurricanes, tornadoes that shove things, hail, dust storms and tides join the existing storms. | Partial: F13 thunderstorms, blizzards, drought, blood rain | M | ★★★ | D11 | Bloat#288, Arc Weather |
| WLD-12 | Old battlefields remembered | Where a big fight happened bones stay, the place is named, and dwarves honour, guard or shun it (ghosts are possible). | Missing | S | ★★ | O09 | Core69, Story:Battlefield Lunch |
| WLD-13 | Ages named by the world | The age's name changes with the world's state (an age of dragons, an age named for your people if they dominate). | Missing | S | ★★ | O09 | Log2000s (Ages named by the world), Story:Tales Foretold: Queens and Kings |
| WLD-14 | Region histories | A region turning evil is a dated event and its old name is kept. | Missing | S | ★★ |  | Bloat#239, roadmap Region histories |
| WLD-15 | Region facts by discovery | Each region lists the creatures and resources the fort has actually seen there (a bestiary that fills in). | Partial: Regions legends tab | S | ★★ |  | Bloat#209, Core94 (bestiary) |
| WLD-16 | Deep snow and rain puddles | Snow piles deep enough to wade through and lingers; rain collects in hollows and dries. | Partial: snow lies until spring; drought dries shallow water | S | ★★ | D11 | Bloat#292, Bloat#286, roadmap Snow, ice and rain |
| WLD-17 | Footprints and tracks | Creatures leave tracks in sand, snow and blood that fade; hunters and sheriffs can follow them. | Missing | S | ★★ |  | Bloat#321, Story:The Beast |
| WLD-18 | Forgotten lore | History not recorded in books or engravings fades; a fresh player (or overseer) rediscovers it through ruins, books and tales. | Missing | M | ★★ | O09 | Bloat#261, roadmap Legends you can search, Story:Dog-man |
| WLD-19 | Towns fall and rise | Sites are destroyed and rebuilt during play; a conquered site is razed, plundered, made tributary, given a puppet ruler or annexed. | Partial: sacks and conquests in worldgen history | M | ★★ | B17 | 2016 villains, Log2000s (Five fates for a conquered site) |
| WLD-20 | Alliances against doom | Peoples threatened by a necromancer or demon band together, which can bring allied armies past the fort. | Missing | M | ★★ | B17 | Log2020s (Alliances against doom), Log2000s (Alliances against evil) |
| WLD-21 | Refugees and bandit camps | People displaced by war wander, settle near safe forts or turn bandit in ruins and caves. | Missing | M | ★★ | B17 | Log2000s (Refugees and bandits) |
| WLD-22 | Artifacts circulate | Worldgen makes masterworks, heroes' named weapons and relics that pass by theft, gift and looting. | Partial: one artifact in the sealed ruin | M | ★★ |  | 2016, 2020 artifacts |
| WLD-23 | Ruins and tombs by culture | Each people buries its dead its own way; fallen peoples leave ruins and grand tombs with treasure and traps. | Partial: F08 sealed marble hall | M | ★★ |  | Core67-68, 2011 Maps I.C |
| WLD-24 | Necromancers spread | Unchecked necromancers can empty whole regions late in history, raising the threat to the fort. | Partial: F14 one nearby necromancer | M | ★★ |  | Log2020s (Necromancers vs the world) |
| WLD-25 | Time passes between forts | Starting a new fort in the same world moves history on years, so old forts' fates play out. | Missing | M | ★★ | P01 | Log2000s (Time passes between forts) |
| WLD-26 | Companies and orders in history | Merchant and mercenary companies, military orders and craft guilds exist in the world, rise, re-arm and go bust. | Missing | M | ★★ |  | Log2020s (Companies and orders), Log2000s (Mercenary companies, Merchant companies) |
| WLD-27 | Religious demography | Faiths are tracked by membership, build shrines and holy cities, and persecute rivals. | Missing | M | ★★ |  | Log2020s (Religious demography) |
| WLD-28 | Demographic pressure | Towns short of women go seeking; towns with too many young men grow warlike. | Missing | M | ★★ |  | Bloat#193, Bloat#194 |
| WLD-29 | Seashores | Crabs, gulls, jellyfish, shells, tide pools, driftwood and flotsam from far peoples on coastal sites. | Partial: coastal site feature | M | ★★ |  | Bloat#328, roadmap Swamp and shore life |
| WLD-30 | Knowledge that spreads and is lost | Each people knows crafts, law and philosophy to a degree; knowledge spreads, is lost and is rediscovered. | Missing | L | ★★ |  | Bloat#27, 2016 innovation system |
| WLD-31 | Villains in the world | Historical figures run bandits and agents from hideouts, raid, and plot theft, kidnap and assassination. | Planned: B04 | L | ★★★ |  | 2016 villains, Log2020s (Historical villainy) |
| WLD-32 | A fully living world | Births, marriages, succession and migration run on real people, so migrants and invaders are figures with pasts. | Missing | XL | ★★ | B17 | Core77, Log2000s (Living world during play) |
| WLD-33 | World-spanning features and gentler land | Huge features across the world above and below ground; smoother height changes. | Partial: ranges, rivers, relief follows world | L | ★★ |  | Bloat#320, Bloat#366 |
| WLD-34 | Roads and tunnels between sites | Peoples build roads, bridges and tunnels, shown on the world and embark maps. | Planned: B01 | M | ★ |  | Req379, Log2000s (Roads and tunnels between sites) |
| WLD-35 | Graves of the great, summits climbed | Notable dead get findable graves, office-holders monuments; reaching a summit is a legend. | Partial: tombs, monument | S | ★ |  | Log2000s (Graves and monuments, Climbing summits) |
| WLD-36 | Finer world clock | World events scheduled week by week so legends read in detail. | Partial: yearly history | S | ★ |  | Log2000s (Weekly world events) |
| WLD-37 | Map dressing | Seasonal browning, named grasses, lake plants, foam around river rocks, straighter rivers, no beaches on glaciers, cliffs hiding the sky. | Partial: seasonal trees, biome plants | S | ★ |  | Bloat#208, Bloat#318, Bloat#290, Bloat#352, Bloat#284, Bloat#334, Bloat#365, Bloat#235 |
| WLD-38 | Languages and scripts | Generated languages with grammar and sounds, and each people's own writing glyphs. | Partial: generated names | L | ★ |  | Bloat#348, Bloat#187, Core97-99 |
| WLD-39 | Deeper world settings | Dials for magic, hostility and realism, down to a waterless or wholly peaceful world. | Partial: savagery, minerals, size, history | M | ★ |  | Arc World-gen Parameters, 2020 |
| WLD-40 | Sites with substance | Kings' retinues exist in history, vassal sites keep their people, dwarf and elf towns are real places. | Partial: generals from history | M | ★ |  | Bloat#115, Bloat#371, Bloat#206 |
| WLD-41 | Tree lore | Peoples know their trees; evil lands grow nasty ones only goblins know. | Missing | M | ★ |  | Bloat#111 |
| WLD-42 | Editors | Draw maps, sites, creatures and histories by hand. | Partial: seeds | L | ★ |  | Arc Editors, 2020 |
| WLD-43 | Ages of myth and after | Start in a mythic age, or play on into ordinary times after the great beasts are gone. | Missing | XL | ★ |  | Bloat#148, Bloat#149, Arc Early/Late Game |
| WLD-44 | Other planes | Worlds and planes linked to this one (dream, underworld, faerie). | Missing | XL | ★ |  | Arc Planes, PG81 |

*Dropped or moved: worldgen settings and living worldgen screen (Have, W21), embark site info and mesas (Have); legend filters/browser moved to UI-18; heredity and inherited looks (Story WLD#11, Bloat#347) left to Society; "Claim any cave" and old-fort contents moved to OUT-09/OUT-11; human-town, wizard and elf-retreat modes moved to OTH-10.*

## Adventure-ish / outside the fort (OUT)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Squad missions | From the world screen: raid, pillage, raze, demand tribute, rescue, recover artifacts, request or send dwarves | `e` sends a squad to a Ruin, a Raid or a Beast lair; odds from strength vs danger (`game/expeditions.rs`, F09) | No rescue, recovery, tribute or conquest |
| Travel | About a month's march, the route drawn on the map on return | Days away, abstract; no route shown | Small |
| Mission outcome | Report with loot, captives, deaths, the tactician named; one loot item per raider; site depletion | Loot, epithets, legends, or losses | No captives taken by either side, no depletion |
| Consequences | Raided civs retaliate; enemies' kin take revenge; fame spreads | None: expeditions have no comeback | Missing |
| Captives and rescue | Raiders and kidnapped children can be held; rescue missions follow | None | Missing (S03 captives step open) |
| Artifact quests | Patrons ask for artifacts; your artifacts are targets; squads recover stolen ones | None | Planned B05 |
| Outposts and territory | Hamlets founded near a prosperous fort; conquered sites send tribute; messengers to outposts | None | Planned B03 |
| Reclaim | Retake your own fallen fort (or any cave) with its corpses, items and ghosts | Ruin kept as a save slot only (`saves.rs`, `ui.rs`) | Large |
| Retire / unretire | Leave a fort running in the world and return later | None | Large |
| Persistence across forts | Survivors of old forts turn up as migrants; adventurers visit old forts | None | Missing |
| Reputation | Fame with peoples and families brings gifts and revenge | Heroes admired; word.rs spreads lacks | No fame abroad |
| Inbound adventurers | Questers, monster slayers and mercenaries come to the tavern | Bards and wandering warriors (`game/visitors.rs`) | F07 open |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| OUT-01 | Expeditions leave captives behind | A beaten squad can lose members alive to the enemy instead of dead, named in the report and remembered by family. | Missing | S | ★★★ | B18 | Log2000s (Captured raiders), Story:catalogue (Captives and rescue) |
| OUT-02 | Rescue missions | A squad goes after a named captive (from a lost raid, a kidnapped child, a seized noble); success brings them home changed. | Planned: B03 (freeing prisoners) | M | ★★★ | B18 | Bloat#44, roadmap Rescue missions, Story:Snatcher, Log2000s (Squad raids, rescues and recoveries) |
| OUT-03 | Retaliation for raids | Raiding a civ's site, even fellow dwarves, brings a counter-raid sooner and says why. | Missing | S | ★★★ | B18 | Log2000s (Retaliation for raids), Log2020s (Real populations, real grudges) |
| OUT-04 | Proving trials | A recruit or child of age is sent alone on a small task off-map; success earns a name, failure shame or death. | Missing | S | ★★★ |  | Story:Cado's Magical Journey, Story:Monster Quest |
| OUT-05 | Tall tales from returning squads | Returning members tell their deeds in the tavern, embellished; listeners' opinion of them and the fort's fame shift. | Partial: bards sing legends | S | ★★★ | F07 | Bloat#196, Bloat#192, Story:The Adventures of Gornon |
| OUT-06 | Recover stolen artifacts | When an artifact is stolen, a squad can be sent after it; it returns with a tale or only rumours of where it went. | Planned: B05 | M | ★★★ | F07 | 2016, 2020 artifacts, roadmap Recover what was stolen, Story:Heroes of the Coast |
| OUT-07 | Treasure trails | Map fragments, tablets and books found in ruins or bought from traders point to a hidden site, opened as a new expedition target. | Partial: the Whispering Stone leads to the sealed hall | M | ★★★ | F07 | Treasure Hunter role, PG32, PG134, Story:Curse of the Cyclops |
| OUT-08 | Bandits on the road | Outlaw camps toll or rob caravans and expeditions; they can be hunted, paid off or ignored. | Missing | M | ★★★ | P03 | Log2020s (Bandit harassment), Log2000s (Bandit robberies), Story:Outlaws of Norville, Caravan Arc III.B.1 |
| OUT-09 | Reclaim a fallen fort | A new expedition retakes an earlier ruin in the same world, with its corpses, loot, traps, ghosts and whatever moved in. | Partial: ruin kept as a save slot | L | ★★★ | P01 | Log2000s (Reclaim a fallen fort), Core70, Req266, Arc Ghosts, Bloat#182, Bloat#20, 2016, Story:Features page |
| OUT-10 | Survivors come back as migrants | Dwarves who fled or outlived a fallen fort turn up in a later fort's migrant waves with their memories and grudges. | Missing | S | ★★★ | P01 | Log2000s (Dwarves from your fallen forts return), Arc Ghosts, 2025 cross-mode |
| OUT-11 | What old forts leave behind | Ruins hold the old fort's records and the overseer's journal, items in ponds and corners, keys to locked doors. | Missing | S | ★★ | P01 | Core70 (Records of production), Bloat#164, Bloat#188 |
| OUT-12 | Fame abroad | Deeds raise the fort's standing with each people and family: gifts, visitors and recruits from friends, revenge from victims' kin. | Partial: heroes admired, epithets | M | ★★★ | F07 | Arc Affiliation, Hero role |
| OUT-13 | Questers come for your artifacts | Visitors in the tavern ask after the fort's artifacts; chatty dwarves let slip where they are; the visitor petitions for it or steals it. | Missing | M | ★★★ | F07 | Log2000s (Questers in the tavern), 2016 artifacts |
| OUT-14 | Aid missions | A friendly civ begs the fort for help against a siege or beast; sending a squad wins standing, refusing costs it. | Missing | M | ★★★ | P03 | PG10 (Pleas for help) |
| OUT-15 | Parley with beasts | An intelligent dragon or demon can be bargained with: tribute for peace, or a quest in exchange. | Planned: B02 (partly) | M | ★★★ |  | Story:catalogue (Parley with beasts), Core93 |
| OUT-16 | Monster slayers | Hunters petition to go after a cavern beast and may die trying; their bodies and gear are found later. | Planned: F07 | S | ★★★ | F07 | Log2000s (Monster slayers), Night-creature role |
| OUT-17 | Companions turn back | Expedition members refuse a mad venture or quit after losses, by loyalty and courage, and come home early. | Partial: S06 step "losing squad returns early" open | S | ★★ | S06 | Story:Mother of Death, Core24, Core61 |
| OUT-18 | Coming home | Returning members react to home: reunions, grief at deaths while away, shock at changes. | Missing | S | ★★ | B18 | Story:Warriors of the Dead |
| OUT-19 | Field rations and care packages | Squads pack food and drink by setting; the fort can send supplies to a squad in the field. | Missing (fits S06) | S | ★★ | S06 | Log2020s (Field rations), Bloat#175 |
| OUT-20 | The tactician matters | The squad's best tactician tilts off-map battle odds, gains skill from it and is named in the report. | Missing: odds use strength only | S | ★★ | S06 | Log2000s (Military tactics skill) |
| OUT-21 | Pillage depletion | Each raider brings back one loot item or animal; a raided site empties and recovers slowly. | Partial: expeditions bring loot | S | ★★ | B18 | Log2000s (Pillage depletion) |
| OUT-22 | Routes on the map | Mission travel takes real days by distance, and the route is drawn on the world map when the squad returns. | Partial: days away, no route | S | ★★ | B18 | Log2000s (Squad raids, rescues and recoveries) |
| OUT-23 | Scouting missions | Unexplored world cells stay hidden until a scout goes; discoveries get names (fords, peaks) and become targets. | Missing | M | ★★ | P03 | Explorer role |
| OUT-24 | First contact | An exotic visitor from an unmet people gets an audience; gifts are exchanged and relations start. | Missing | S | ★★ |  | Explorer role, Bloat#73 |
| OUT-25 | Tracking to the lair | Hunters follow tracks of a beast or thief, revealing its lair as an expedition target. | Missing | M | ★★ |  | Story:The Beast, Story:Monster Quest, Bloat#321 |
| OUT-26 | Pack animals and mounts | Expeditions take mules or other beasts to carry loot and travel faster. | Partial: F09 expeditions | S | ★★ |  | Log2020s (Pack animals and mounts), Bloat#195 |
| OUT-27 | Dreams on the road | Away from home, a dwarf dreams a vision that points to a site or a danger. | Missing | M | ★★ |  | Bloat#228 |
| OUT-28 | Talk of the fort's buildings | Visitors carry home word of the fort's architect, monument and materials, raising its fame. | Missing | M | ★★ | F07 | Bloat#378 |
| OUT-29 | Goblin pits as targets | Goblin towers have tunnel networks and death pits below, a deep and deadly raid target. | Partial: raids on dark fortresses | L | ★★ |  | Bloat#383, PG100 |
| OUT-30 | Outposts and attached hamlets | The fort founds work camps or hamlets, or nearby villages attach; they send goods and migrants and can be raided. | Planned: B03 | L | ★★ | P03 | Core28-29, Arc County, Log2000s (Settlements grow around the fort), 2016-2020 hill dwarves, Core84-86 (homesteading) |
| OUT-31 | Messengers between sites | Birds or runners carry word between the fort, its outposts and squads in the field. | Missing | S | ★★ | P03 | Story:The Assassination of Zecalo Bronzeflower, Log2000s (Messengers to outposts) |
| OUT-32 | Retire and unretire | Leave a fort running in the world and come back years later to find what became of it. | Missing | L | ★★ | P01 | Log2000s (Retire and unretire forts), Log2020s (Retire and reclaim, Quests and visiting old forts), Story:Features page |
| OUT-33 | Trophies and desecration | Squads bring home enemy trophies or desecrate the fallen; boasts and outrage both go into history. | Missing | M | ★★ |  | Bloat#161 |
| OUT-34 | Watch the off-map fight | A mission's battle is shown as a short abstract skirmish instead of a dice roll. | Missing | L | ★★ |  | Log2020s (Tactical party control) |
| OUT-35 | Guides | Expeditions into unknown land do better with a guide; without one they may wander. | Missing | S | ★ |  | Story:catalogue (Guides) |
| OUT-36 | Collector requests | Collectors abroad want rare items (coins, insects, fine bottles) and pay for them. | Missing | M | ★ |  | Bloat#163 |
| OUT-37 | Party discipline | Expedition members don't turn on each other in an ambush. | Partial | S | ★ |  | Log2020s (Party discipline) |

*Dropped (adventure-only or engine trivia): learning to sneak (#159), eating vermin (#160), puzzle boxes (#171), riding positions (#327), open conversations and phrasing (#200-#205), shop names (#210), sprinting (#214), pits and leaps (#227), weather talk (#266), buy anything (#267), seeing who's who (#307), unknown jobs (#351), holding breath (#357), top-level swimmers (#369), reading people (#374), strangers' privacy (#375), thief and disguise play (Thief role), conversation as a game (Log2020s), playing other peoples (moved to OTH-10).*

## Interface & quality of life (UI)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Alerts | Unmissable popups for the worst news; each announcement set to pause, recenter or neither; combat reports as icons | Spotlight cards pause and turn the camera (`game/spotlight.rs`); overseer cards pass on their own | No per-type control |
| Reports | A persistent, prunable log; separate combat, sparring and hunting reports | `r` reports with filters (`ui.rs`); batch reports with blow counts (`report.rs`) | No fight log in play |
| Fort mood at a glance | A row of faces, one per dwarf; unit list sortable by stress | Dwarf glyph `☺`/`☻` on map; status screen health (`ui.rs`) | No mood strip |
| Dwarf sheet | Thoughts with causes, memories by season, medical history, live-updating pane | Dwarf screen with thoughts, needs, family (`ui.rs` Unit mode) | No life story, not live |
| Camera | Recenter hotkeys, named spots, follow a unit, pop to surface/deepest point, clickable minimap | Cursor follow, spotlight camera, mouse pan, world minimap; no fort minimap | W25 open |
| Watching autoplay | (not a DF feature) | `A` hand-over, Watch a fort, overseer notes pane, speeds 1-60 (`app.rs` SPEEDS), brain `B` | No replay, no highlight reel, no director camera |
| Legends and chronicle | Clickable browser, filters by creature, period, place | `L` tabs, `c` chronicle, game-over story page | No filters or links |
| Designation and building | Paint and rectangle, priorities, blueprint mode, size while dragging, fast material pick | Box designations and constructions, two-column build menu | No priorities, no size shown |
| Lists | Places, objects, stocks, units by category, all searchable with recenter | Status tabs, labor grid, units list | No search, few recenters |
| Depth | Several levels shown shaded below, liquid depths, ramp arrows | One level at a time | Missing |
| Settings | Difficulty tabs set at embark, in-game options, saved menu presets | CLI flags and world settings only | Missing |
| Embark | Pick items and animals, site finder by metal/aquifer, neighbours' stance | Fixed loadout; site features and aquifer shown | Partial |
| Help | Tutorials, tooltips, encyclopedia | `?` help, hover tooltips | Partial |
| Export | Screenshots and whole-map export | None | Missing |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| UI-01 | Mood strip | One face per dwarf in a strip along the top, coloured by stress, so the fort's mood reads at a glance; click one to jump to the dwarf. | Missing | S | ★★★ | O05 | Log2020s (Mood strip) |
| UI-02 | Last words | A dying dwarf's final thought goes into the announcements, the spotlight card and the chronicle. | Missing | S | ★★★ | O05 | Story:catalogue (Last words) |
| UI-03 | Director camera | While watching, the camera follows a chosen dwarf or cuts to the most dramatic thing happening (fight, fire, mood), with a lock key to stay put. | Partial: spotlight turns the camera; cursor follow | S | ★★★ | O05 | Log2000s (Camera follow), Bloat#277, Req434 |
| UI-04 | Highlight reel and replay | Record the fort's big moments (map snapshots around each spotlight) so a finished autoplay fort can be replayed as a fast-forward reel. | Partial: siege_replay test | M | ★★★ | O05 | Req43 (Replay recorder) |
| UI-05 | A dwarf's life story | The dwarf sheet lists recent thoughts with their causes and a life story of memories by season and year, plus medical history. | Partial: thoughts on the dwarf screen | S | ★★★ | O06 | Log2020s (Thoughts and memories tabs) |
| UI-06 | Sound-based reports | Far-off events come in as sounds ("screams from the north cavern", "clash of steel near the depot") that expand to detail on demand. | Partial: spotlight cards | S | ★★ | O05 | Bloat#277, Core21, Story:catalogue (Sound-based reports) |
| UI-07 | Announcement control | Each kind of announcement can pause, recenter, show or hide; building completions announced. | Partial: big moments pause | S | ★★ | O05 | Req434, Req79, Req207, Log2000s (Configurable announcements) |
| UI-08 | Death scenes in the chronicle | Notable dwarves' deaths told in a line each on the chronicle page. | Partial: deaths and causes recorded | S | ★★ | O06 | Story:catalogue (Death scenes) |
| UI-09 | Fight log and battle pages | A readable blow-by-blow log for each fight, gathered into a page per named battle. | Partial: blow counts in batch reports | S | ★★ |  | Bloat#270 |
| UI-10 | Export the fort | One key writes the current map (or every level) as text/HTML and the fort's story as a file, to share the ruin. | Missing | S | ★★ | O05 | Bloat#268, Req337 |
| UI-11 | Stuck-path warnings | Announce creatures whose path keeps failing (and throttle them). | Missing | S | ★★ | X03 | Log2020s (Stuck-path warnings) |
| UI-12 | Job-done announcements | Say when every reachable dig or felling designation is done. | Missing | S | ★★ | X03 | Bloat#199 |
| UI-13 | Unit list with stress | Units sorted by stress, happiness or job; categories for citizens, pets and livestock, visitors, dead or missing. | Partial: units list | S | ★★ | O06 | Log2020s (Unit list with stress), Log2000s (Unit list categories), Bloat#153 |
| UI-14 | Live unit sheet | Clicking a creature opens a pane that updates while unpaused; several things on a tile become tabs. | Partial: look with Tab cycling | S | ★★ | O06 | Log2020s (Live unit sheet) |
| UI-15 | Family tree screen | Marriages, parents, children, siblings and pets with zoom-to. | Partial: family on the dwarf screen | S | ★★ | O06 | Bloat#51, roadmap Family tree screen, Log2000s (Relationship screen) |
| UI-16 | Fort rank and wealth breakdown | The top bar shows the fort's rank; hovering breaks wealth into categories and says what the next title needs. | Partial | S | ★★ |  | Log2020s (Fort rank and wealth tooltip), Req388 |
| UI-17 | Map notes | Pin notes with a symbol to tiles on the fort and world maps; later reusable as patrol waypoints. | Partial: overseer notes; players can't | S | ★★ | W25 | Bloat#355, Log2000s (Rectangle construction and map notes) |
| UI-18 | Legend browser | Filter legends by creature, place, period or event type; names are links; nicknames recorded. | Partial: tabs | S | ★★ |  | Bloat#379, Log2020s (Legends browser), roadmap Legends you can search |
| UI-19 | Depth display | Show a few levels below shaded by depth, numeric liquid depths, ramp arrows, and what lies under water. | Missing | S | ★★ | W25 | Log2020s (Depth display), Bloat#367, Bloat#326 |
| UI-20 | Settings and difficulty | A settings screen with difficulty knobs (attack frequency, thieves, beasts, wealth triggers, dig speed) set at embark, menu presets saved between games. | Missing | S | ★★ | X03 | Log2020s (Settings and difficulty tabs), Req548, Bloat#354 |
| UI-21 | Places and objects lists | Tabs for zones, stockpiles, workshops, farms; objects grouped as artifacts and named items; each with recenter. | Partial: status screen | S | ★★ | W25 | Log2020s (Places and objects lists) |
| UI-22 | Rumours screen | Everything the fort has heard (plots, beasts, wars) in one place, with source and trust. | Planned: B04 | M | ★★ |  | Story:catalogue (Rumours screen) |
| UI-23 | Intrigue board | A conspiracy board of people and plots that fills in as evidence comes. | Planned: B04 | M | ★★ |  | Log2000s (Intrigue board), Log2020s (Traitors and heists) |
| UI-24 | See the army coming | The approaching army shown on the world map with size and kind. | Planned: B01 | M | ★★ |  | Story:catalogue (See the army approach) |
| UI-25 | Equipment readiness grid | Soldiers by gear slot, coloured has / assigned not collected / can't find. | Partial: uniform weapon + armour on/off | S | ★★ |  | Log2020s (Equipment readiness grid) |
| UI-26 | Designation tools | Free paint and rectangle, dig priorities, circle and saved shapes, size shown while dragging, icons over forbidden/dump items. | Partial: boxes | S | ★★ | X03 | Log2020s (Designation paint), Bloat#314, Bloat#315 |
| UI-27 | Fast building placement | Categories, filterable materials, nearest or last material auto-picked, keep placing after one. | Partial | S | ★★ | X03 | Log2020s (Fast building placement) |
| UI-28 | Stocks window and search | A filterable stock list with recenter, forbid and dump; type to filter any long list. | Partial | S | ★★ | W25 | Log2020s (Stocks window), Log2000s (Searchable lists) |
| UI-29 | Why can't it be done | Workshop task tooltips show product and inputs; impossible jobs and soldiers say exactly what's missing. | Partial | S | ★★ | X03 | Log2020s (Workshop task tooltips), Req390, Req430, Req546, Log2000s (Missing reagents, Detailed refusal reasons) |
| UI-30 | Embark loadout and site finder | Pick embark items and animals; search sites by metal, sand, aquifer; see neighbours' stance and resulting professions. | Partial: fixed loadout, features shown | M | ★★ |  | Log2020s (Embark loadout), Log2000s (Embark notes and site finder, Embark profiles), Req432, Bloat#361 |
| UI-31 | Recenter hotkeys and fort minimap | Saved camera spots, jump to surface or deepest point, clickable fortress minimap. | Planned: W25 | S | ★ | W25 | Log2020s (Recenter hotkeys, Clickable minimap), Bloat#197 |
| UI-32 | Step size | Advance by one tick, an hour or a day, for close watching. | Partial: six speeds, pause | S | ★ | O05 | Bloat#273 |
| UI-33 | Sound and music cues | Optional terminal bell or audio for sieges, deaths and moods; music by context. | Missing | M | ★★ |  | Core21, Log2020s (Music by context) |
| UI-34 | Quick hops | Jump between linked unit, item, job and building views; inventory on the profile; assign rooms from the dwarf screen. | Partial: Enter opens a dwarf | S | ★ | W25 | Bloat#52, Bloat#118, Bloat#311, Bloat#70, Req411 |
| UI-35 | Status readouts | Each animal's and patient's state, next treatment, injured parts coloured by severity. | Partial: wounds listed | S | ★ | O06 | Log2000s (Pasture and medical status readouts, Injury colour readout) |
| UI-36 | Justice screen and theft totals | More detail on crimes and how much thieves have taken. | Partial: crimes on nobles screen | S | ★ |  | Bloat#106, Bloat#107 |
| UI-37 | Editable orders | Type a new count, reorder the list. | Partial | S | ★ |  | Log2020s (Editable orders) |
| UI-38 | Confirm the dangerous | Disbanding squads, clearing hotkeys and deleting saves ask first. | Partial: save delete confirms | S | ★ |  | Log2020s (Confirm the dangerous stuff) |
| UI-39 | Custom names, colours, glyphs | Rename dwarves, squads and zones; colour by profession or clothing; custom glyphs and profession names. | Missing | S | ★ |  | Bloat#274, Bloat#121, Bloat#368, Bloat#349, Req564, Log2020s (Profession vs clothing colours), Log2000s (Customisable nicknames) |
| UI-40 | Contents at a glance | Barrels and bins show what's inside; pack animals' loads. | Missing | S | ★ |  | Log2020s (Contents at a glance) |
| UI-41 | Burrow pause | Switch a burrow on or off without alerts; burrow symbols and descriptions. | Partial: burrows | S | ★ |  | Log2020s (Burrow pause) |
| UI-42 | Location info | Temple and meeting-hall boxes show their god and type; petition status. | Missing | S | ★ |  | Log2020s (Location info) |
| UI-43 | Sort saves by date | Save list by last played. | Partial | S | ★ |  | Log2020s (Sort saves by date) |
| UI-44 | Pet ages and profiles | Show a pet's age and profile. | Partial: named pets | S | ★ |  | Bloat#67 |
| UI-45 | Noise map | Show where noise that wakes sleepers comes from. | Missing | S | ★ |  | Bloat#303, Bloat#88 |
| UI-46 | Debug options | In-game tweaks for internal settings. | Partial: F3, TRIAL_PROF, SPIKE_MS | S | ★ |  | Bloat#283 |
| UI-47 | Guides and encyclopedia | Starter tutorial on a sensible site, dismissable how-tos, encyclopedia. | Partial: `?` keys | M | ★ |  | Core19, Core58, Arc Advanced Help, Log2020s (Guides and tooltips) |
| UI-48 | Macros | Record key sequences. | Missing | M | ★ |  | Bloat#358 |
| UI-49 | Translations | All text moved out for translators. | Missing | M | ★ |  | Core54 |
| UI-50 | 3D views | Optional views of the land and the fort's levels. | Missing | L | ★ |  | Bloat#326 |
| UI-51 | Graphics and tilesets | Tilesets, window resizing, coherent mouse UI. | Partial: truecolor, mouse | L | ★ |  | Core50-52, Bloat#121 |

*Dropped as already done: history screen (#108), container paging (#119), squads from the main screen (#154), map seeds (#217), big-alert popups (Have: spotlight cards). The "advisors" idea (Core22) is in OVR-22.*

## Other (OTH)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Starting scenarios | Choose what the fort is for (frontier, religious site, prison colony, mining company, citadel, inn, palace), each changing migrants, caravans, goals | One embark kind (`game.rs` embark) | Planned B10 |
| Secret content | Hidden fun stuff nobody is told about | Mysteries (F16), sealed ruins (F08) | Partial |
| Tuning thresholds | Attack, megabeast and noble triggers editable in data; invasion caps | Hard-coded; CLI style/seed/years for batches (`main.rs`) | Missing |
| Data-driven content | Raws, scripted generators, mods as packages | Rust enums (`creature.rs`, `items.rs`) | Missing (by design) |
| Arena / test harness | Arena for fights | `arena`, `siege_replay` ignored tests | Have |
| Pathing performance | Throttled failing paths, true distances | Flat-array pathfinding, F3 profiling | Partial |
| Ambient simulation | Smells, wind, forest floor, tripping | Miasma drifts; weather | Partial |
| Other peoples | Play a human town, elf retreat, wizard's tower | Dwarves only | Missing |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| OTH-01 | Starting scenarios | Pick the fort's purpose (frontier, mining company, citadel, prison colony, inn, religious site, refugee band, childhood friends); it shapes migrants, caravans, goals and suits an overseer style. | Planned: B10 | M | ★★★ | B10 | Core76, Req214, 2016-2020, Story:Outlaws of Norville, Story:catalogue (Starting scenarios), Bloat#143 |
| OTH-02 | Why you came | The embark carries a backstory and starting effects (fleeing a war, sent in disgrace, chasing a rumour) that colour the first years. | Planned: part of B10 | S | ★★★ | B10 | Core76 |
| OTH-03 | Hidden fun stuff | Rare secret surprises nobody is told about, found only by digging or waiting. | Partial: F16 mysteries, F08 ruins | M | ★★★ |  | Core66, Req135, Bloat#61, Bloat#4 and other title-only entries, Log2000s (Hidden fun stuff) |
| OTH-04 | Tunable thresholds | Siege, megabeast, thief and noble triggers and invasion caps live in a small data file (or CLI flags) so batches can test balance. | Missing | S | ★★ | S10 | Log2000s (Thresholds in data, Threat thresholds you can tune, Invasion size caps), Log2020s (Siege difficulty knobs) |
| OTH-05 | Tripping | Running over rough ground, mud or ice, or while drunk, can trip a dwarf. | Missing | S | ★★ |  | Story:catalogue (Tripping) |
| OTH-06 | Path throttling | Creatures that keep failing to path try less often. | Partial | S | ★★ | S10 | Log2020s (Path throttling) |
| OTH-07 | Smells | Burning flesh, rot and cooking smelled at a distance, drawing beasts or warning dwarves. | Partial: miasma | S | ★ |  | Story:catalogue (Smells) |
| OTH-08 | Wind | Wind rattles doors, chills the outdoors and carries smoke and fire. | Partial: weather | M | ★ |  | Story:catalogue (Wind) |
| OTH-09 | Forest floor | Stumps after logging, leaf litter, fungus on logs, bodies slowly covered. | Missing | S | ★ |  | Story:Root, Story:catalogue (Forest floor) |
| OTH-10 | Other peoples' settlements | The overseer runs a human town, an elf retreat or a wizard's tower with its own rules. | Missing | XL | ★★ |  | Bloat#146, Bloat#147, Bloat#25, roadmap Play other peoples |
| OTH-11 | Data-driven content and mods | Creatures, items, materials and colours in editable files; scripted generators; mods as versioned packages. | Missing | L | ★ |  | Story:catalogue (Moddable definitions), Log2020s (Scripted generators, Mods as packages), Log2000s (Custom workshops and reactions), Bloat#323 |
| OTH-12 | Smooth item ageing | Items age and rot smoothly across any span, including time skipped between forts. | Partial: rot over days | M | ★ |  | Bloat#220 |
| OTH-13 | Lasting wilds | Wilderness the fort has visited is saved and stays changed. | Missing | M | ★ |  | Bloat#317 |
| OTH-14 | Site sprawl | Huge sites split into linked map pieces with creatures moving between them. | Partial: `M` grows the map | L | ★ |  | Bloat#198 |
| OTH-15 | Owned liquids | Some liquids belong to someone (a dwarf's barrel, a noble's pond). | Missing | S | ★ |  | Bloat#122 |
| OTH-16 | Melting plants | Plants of exotic materials melt instead of burning. | Missing | S | ★ |  | Bloat#256 |
| OTH-17 | True distances | Range checks use real distance, not squares. | Partial | S | ★ |  | Log2020s (Euclidean distances) |
| OTH-18 | Flat world option | A flat world to save memory. | Missing | S | ★ |  | Bloat#299 |

*Dropped as engine trivia: #101, #120, #130, #211, #215, #226, #336, #341, #342, #100, #157, #248; multithreading and arena (Have).*

## The overseer (AI player) (OVR)

### Dwarf Fortress and DwarfClone

| Aspect | Dwarf Fortress | DwarfClone today | Gap |
|---|---|---|---|
| Who runs the fort | The player, above an expedition leader, then an elected mayor and appointed nobles | A disembodied overseer with a name and traits; leader/mayor are separate nobles (`overseer.rs`, `dwarf.rs`) | Overseer can't die, be deposed or succeeded |
| Advice | Advisors / a council suggesting what the fort needs (planned, Core22) | Troubles and status shortages feed it; notes in its voice | No council with its own agendas |
| Diplomatic choices | Pay or refuse tribute, hand over an artifact, abuse or honour envoys, answer pleas, send messengers | None | B02 open ("overseer decides by traits") |
| Justice choices | Accuse suspects, choose punishments, banish, execute, interrogate | Sheriff acts automatically (`justice.rs`) | No overseer judgement |
| Civic choices | Accept or refuse migrants, petitions, residents, a barony; pick the baron | Automatic | None of these decisions exist |
| Crisis choices | Martial law, emergency levy, surrender | Shelter burrow, bridge, squad station (`ov_alert`) | No last-resort options |
| Learning | Player learns from each siege | S04 step "learns after a siege" open | Planned |
| Ambitions from the world | Monarch's Mountainhome goal, patrons, blueprints by inspired engineers | 9 ambitions chosen seasonally | No outside-sourced ambitions |
| Expressing personality | (the player is the personality) | Traits sway ambitions, layout, gate timing, army size, note voice | Few decisions where traits show |
| Watching and nudging | (n/a) | Brain screen, nudges, force, overseer notes pane, cards | No intent line, no nudge from a card, no hand-picked traits (O01) |
| Legacy | Records and diaries found in a ruin | Notes capped at 60; chronicle; game-over page | Overseer's record lost |

### Ideas

Best first: fun for an overseer-run fort, then how easy it is.

| ID | Idea | Mechanic | Status | Size | Fun | Roadmap | Sources |
|---|---|---|---|---|---|---|---|
| OVR-01 | Parley at the gate | A besieging army halts and demands tribute, an artifact, a noble or a criminal; the overseer pays or refuses by traits (Greedy hoards, Cautious pays, Bold refuses) and the note says why. | Planned: B02 (decide by traits) | M | ★★★ | B02 | 2020 artifacts, 2025 diplomacy, 2016 villains, Log2000s (Parley at the gate), Log2020s (Artifact bribe), Story:Dwarven Assault, Story:catalogue (Ransom demands, Parley and white flag) |
| OVR-02 | A mortal overseer and succession | The overseer is a real dwarf (the expedition leader, later the mayor or baron); if it dies, is captured or deposed, an heir or election brings a new overseer with new traits, and the chronicle marks the reign. | Missing | M | ★★★ | O07 | Core63, Req587, 2011 III.D, Log2000s (Succession and claims, Leaders away or captured, Elections and complaints), Story:catalogue (Claimants and proof), 2020 villainous plots (assassins) |
| OVR-03 | The council lobbies | Post-holders and nobles each push an agenda (the commander wants walls, the broker trade) as a new brain sense; a Stubborn overseer resists, a weak one is steered. | Missing | M | ★★★ | C06 | Story:catalogue (The council lobbies the overseer), Story:Tales Foretold: The Reign of Cenaster, Core22 |
| OVR-04 | Learns from each siege | After a siege it puts traps where the enemy came in, raises a second squad, builds walls, by what killed its dwarves. | Planned: S04 | M | ★★★ |  | Req301 (mirror of attackers learning), Log2020s (Learning from losses) |
| OVR-05 | Spin | A Vain overseer writes a defeat up as a victory in the chronicle and announcements; dwarves who were there may not believe it (a stress thought). | Missing | S | ★★★ | O07 | Story:catalogue (Spin) |
| OVR-06 | The overseer's journal | Every note is kept and bound into a journal page per year beside the chronicle, found in the ruin by a later fort. | Partial: notes capped at 60 | S | ★★★ | O07 | Core70 (Records of production), Story:catalogue (Death scenes) |
| OVR-07 | Martial law and the last stand | When a siege overwhelms the squad, the overseer can arm cooks and haulers or declare martial law; everyone hates it after. | Missing | S | ★★★ | A06 | Bloat#236, Req273, Story:catalogue (Emergency levy) |
| OVR-08 | Banish a dwarf | The overseer can exile a troublemaker or criminal (family follow, friends resent it); the exile may return with bandits or goblins. | Planned: Inbox (banishment) | S | ★★★ | B02 | Log2000s (Expel a citizen), Log2020s (Expulsion), PG47, Story:Brotherly Love, Story:Forest Befouled |
| OVR-09 | Take or refuse the barony | When the fort earns a barony the overseer picks the baron (perhaps itself), or turns it down at a cost to goodwill. | Partial: titles granted automatically | S | ★★★ | B02 | Log2000s (Choose your baron, or refuse) |
| OVR-10 | Traits that drift | Hard memories change the overseer's traits for good (a massacre makes a Bold one Cautious), told in its notes. | Missing | S | ★★★ | O07 | Log2000s (Personality drift from memories) |
| OVR-11 | Petitions | Faiths, guilds and long-staying visitors petition for a temple, guildhall or residency; the overseer answers by traits (Pious accepts temples) or with standing orders, and broken promises anger them. | Planned: B08/B09 | M | ★★★ | B02 | Log2020s (Petitions, Residents vs citizens), Log2000s (Religions petition for temples, Guilds petition for halls, Visitors, residency, citizenship) |
| OVR-12 | Prisoners of war | Captured attackers are ransomed, released, put to work or executed by the overseer's traits, with consequences for the next siege. | Planned: S03 (captives) | M | ★★★ | B02 | 2020, Hero role, Story:catalogue (Surrender and quarter) |
| OVR-13 | Conduct has consequences | A Reckless or Greedy overseer may abuse envoys, raid caravans or fell sacred woods; neighbours remember and turn hostile. | Missing | M | ★★★ | B02 | Bloat#31, Bloat#230, roadmap Abusing envoys, Story:Animal Justice, Story:catalogue (Your cruelty has consequences) |
| OVR-14 | The crown's levy | The monarch demands soldiers for a distant war; the overseer obeys (losing its gate guard) or refuses (losing favour); levied dwarves may come back as veterans or not at all. | Missing | M | ★★★ | B02 | Core33, Story:Tales Foretold: Queens and Kings |
| OVR-15 | Pleas for help | An ally begs for aid; the overseer sends a squad or not, by traits and the fort's danger. | Missing (see OUT-14) | M | ★★★ | B02 | PG10 |
| OVR-16 | Blueprints | An inspired engineer draws plans for a grand work; the overseer may adopt it as its next ambition, and the engineer is honoured or snubbed. | Missing | M | ★★★ |  | Story:Half Dwarf, Story:catalogue (Blueprints) |
| OVR-17 | Floodgates as weapons | A ruthless or desperate overseer opens floodgates or drops magma on invaders (or prisoners) in a sealed section. | Partial: floodgates and levers exist, unused this way | S | ★★★ | A06 | Story:Summoned to Darkness, Story:catalogue (Deliberate flooding) |
| OVR-18 | Faith in the overseer | Dwarves hold an opinion of their leader from losses, hunger and broken promises; low trust brings grumbling, defied orders, food riots and in the end a deposition (see OVR-02). | Missing | M | ★★★ | O07 | Bloat#90, Bloat#177, PG148, PG130, Story:The Adventures of Gornon |
| OVR-19 | Judging crimes | For crimes with no known culprit the overseer accuses a suspect (maybe wrongly) and chooses the punishment (fine, shaved beard, jail, beating) by its traits. | Missing | M | ★★★ |  | Log2000s (Unsolved crimes and accusations), Bloat#75, Core83 |
| OVR-20 | Surrender and occupation | A Cautious overseer facing certain ruin can surrender; the fort lives on under an enemy overlord, paying tribute, until it rises or is freed. | Missing | L | ★★★ | A06 | Log2020s (Surrender that sticks), Log2000s (Occupation, attrition and uprising), Story:The Assassination of Zecalo Bronzeflower |
| OVR-21 | The monarch and the Mountainhome | When the fort is grand and happy enough the monarch moves in, bringing demands and control of the civ's armies and diplomacy; becoming the Mountainhome is a long ambition. | Planned: Inbox (monarch) | L | ★★★ |  | Core30, Req177, Log2020s (The monarch and the Mountainhome), Log2000s (Monarch arrives) |
| OVR-22 | A council's counsel | A short advisors' panel (manager, commander, doctor, broker) lists what each thinks the fort needs; the overseer's choice is shown against it. | Partial: troubles, status shortages, notes | S | ★★★ | C06 | Core22 |
| OVR-23 | Traitorous advisors | A council member works against the fort for gold or a grudge, steering the overseer wrong until found out. | Planned: B04 | M | ★★★ | B04 | Story:Orther Pushes His Luck, Story:catalogue (Traitorous advisors), Log2000s (Corrupt officials) |
| OVR-24 | Watch and nudge | The overseer's next intent shown in one line; nudge it from a spotlight card ("raise the bridge now"); pick its traits by hand. | Partial: brain nudges and forcing; O01 step open | S | ★★★ | O01 | Core22, Req434, Log2020s (Big-alert popups) |
| OVR-25 | Omens sway it | Sky events, prophecies and omens push a superstitious or Pious overseer toward Fortify or Revel. | Missing | S | ★★ |  | Story:Orther Pushes His Luck, Story:catalogue (Omens) |
| OVR-26 | Victory rewards | After a broken siege the overseer may throw a feast for the squad or the whole fort. | Partial: Revel ambition, festivals | S | ★★ |  | Story:catalogue (Rewards after victory), Log2020s (Victory cheer) |
| OVR-27 | Refuse migrants, ask for trades | The overseer turns a migrant wave away when food is short, or asks the liaison for the trades it lacks. | Missing | S | ★★ |  | Bloat#312, Req451 |
| OVR-28 | Neighbour opinion | A sense for how each people sees the fort (felled trees for elves, raids for victims' kin), weighed before chopping, raiding or trading. | Missing | S | ★★ |  | Bloat#72, Req132, Req463, Story:Animal Justice, Log2000s (Retaliation for raids) |
| OVR-29 | Noble displeasure | A sense for nobles' moods: mandates, office envy, favourites; the overseer feeds mandates into work orders or ignores them. | Partial: troubles handle some demands | S | ★★ | C06 | Bloat#78, Bloat#250, Bloat#53, Log2020s (Office envy) |
| OVR-30 | Petitioners at the leader's door | Dwarves queue to complain or settle disputes; the overseer rules, and moods move by its ruling. | Missing | M | ★★ | C06 | Story:catalogue (Petitioners), Log2000s (Elections and complaints) |
| OVR-31 | Hire swords for board | Visiting mercenaries offer service for food and a bed; the overseer takes them on by traits and danger. | Partial: wanderers may join | S | ★★ |  | Log2000s (Mercenaries for board), 2016 taverns, Story:Monster Quest |
| OVR-32 | Appoint a champion | The overseer names one champion of the fort, who leads in battle and is honoured. | Missing | S | ★★ |  | Log2000s (One appointed champion) |
| OVR-33 | Patron works | A rich noble offers to fund a building or statue in their own name; the overseer accepts or not. | Missing | S | ★★ |  | Story:catalogue (Patrons fund works) |
| OVR-34 | Gifts to the ruler | Sending fine goods to one's own monarch earns favour (bigger caravans, a title sooner). | Missing | S | ★★ |  | Req131, Req174 |
| OVR-35 | Favourites and the worst errands | The overseer sends its rivals or the disgraced on the dangerous jobs and keeps its favourites safe; dwarves notice. | Missing | S | ★★ |  | Story:The Order of the Dolphin |
| OVR-36 | Thin seasons explained | The overseer reads why migrants were few (deaths, a slain noble, the fort's name) and reacts. | Partial: word.rs migrant wants | S | ★★ |  | Log2000s (Death drives migrants away) |
| OVR-37 | Missions planned properly | Squad strength, readiness grid, rations and the tactician weighed before an expedition; a losing squad turns back. | Planned: S06 | S | ★★ |  | Log2020s (Field rations, Equipment readiness grid), Log2000s (Military tactics skill) |
| OVR-38 | A "do this now" lever | Job priorities let the overseer push an urgent job (doctor, bridge, cistern) ahead of the queue. | Missing | M | ★★ |  | Req86, Req428, Bloat#54, Log2000s (Job postings and priorities) |
| OVR-39 | Conditional orders | The overseer raises orders with conditions ("brew only if drink below 30") that queue their own ingredients. | Partial: keep-N orders and boosts | M | ★★ |  | Log2020s (Conditional work orders), Log2000s (Work-order conditions), Bloat#253 |
| OVR-40 | Interrogation | The overseer chooses tactics with captured goblins or suspects (intimidate, persuade, flatter); a good one reveals the next plotter. | Planned: B04 | M | ★★ |  | Log2000s (Interrogation), Story:Half Dwarf |
| OVR-41 | Rumours as a sense | Rumours and scout reports feed a sense, so the overseer prepares for an army or beast it has only heard of (and may be fooled). | Missing | S | ★★ |  | Log2020s (Rumours), 2025 scouts |
| OVR-42 | Its life dream | Each overseer carries one life goal (a monument, a hundred dwarves, a slain dragon); reaching it is a milestone. | Missing | S | ★★ |  | Log2000s (Life dreams) |
| OVR-43 | Scenario as charge | The starting scenario sets the overseer's charge and first ambition (a citadel favours Warlord). | Planned: B10 | M | ★★ |  | Core76, 2016-2020 |

