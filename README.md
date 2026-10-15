# DwarfClone

A Dwarf Fortress-style colony simulator for the terminal, written in Rust with ratatui.
You don't control dwarves directly: you pick a site in a generated world, designate work,
lay out zones, queue orders and draft a militia, and the dwarves get on with it according
to their labors, needs, personalities and moods.

## Run

```
cargo run --release            # title screen; new worlds get a random seed
cargo run --release -- 12345   # the first new world uses this seed
```

Use Windows Terminal (or any truecolor + Unicode terminal) at 130x40 or larger.
Press `?` in game for keys.

## Features

**World creation.** New world opens a settings screen: preset, seed, world size (up to
256×128), history length, number of civilizations, savagery and minerals. Then you watch the continent appear and its
history play out year by year (Space hurries it), and accept it or roll another.

**World & history.** A continent of biomes (mountains, hills, forests, grassland,
desert, tundra, swamp, evil-tainted land) with dwarven, human, elven and goblin
civilizations. Two centuries of history are simulated before you arrive: settlements are
founded, wars declared, sites sacked or conquered, rulers succeed one another, megabeasts
ravage towns and heroes slay them. Seas, mountain ranges, forests and plains carry names. Big worlds scroll, with a minimap.
Browse it all in the legends viewer (`L`). You choose
your embark site; the biome shapes the map (relief, forest, lakes, savage wildlife) and
nearby goblins, and any civilization at war with yours in the world's history, decide how
often sieges come.

**Fortress.** A small (64×48), medium (96×64) or large (144×96, 48 levels deep) region, picked
with `s` on the world map: soil over sedimentary and igneous rock, ore veins, gems,
and hidden cavern layers (two, three on deep maps) with pools, tower mushrooms and things that wake when you
break in. Designate mining, channels, stairs, ramps and tree felling. Water falls and
levels out.

**Constructions.** `b`, then `C`: plan walls, floors, stairs, ramps and fortifications
in a box, built from rough stone, blocks or logs. Take them down again to get the material
back. Floors bridge open space, fortifications stop walkers but let water through, and a
roof over surface walls makes a proper room.

**Stone detailing.** Designate (`d`) `s` to smooth natural stone walls and floors and `e` to
engrave smoothed ones. Engravers (the Stone detailing labor, no pick needed) carve scenes from
the world's history, often the fortress's own; look at a tile to read it. Smoothed and
engraved stone adds to a room's value, engravings more the finer they are.

**Crops.** Five crops, each with its seasons and place: stoutcaps (anywhere, all year; eat,
brew, spin), hollowgrain (underground, autumn and winter; brew, mill), threadreed
(underground, spring and summer; spin into thread), sunberries (surface, spring and summer;
eat, brew into wine) and goldoat (surface, summer and autumn; mill). Farmers plant the most
plentiful seed that suits a plot; in look mode, `c` on a farm picks one crop for it.
Eating, brewing, cooking or spinning a plant gives back its seed. A gathering area
(`i`, `w`) laid over wild shrubs lets farmers pick whatever grows there that season.
Dwarves with the Fishing labor fish from the banks of deep water (rivers, lakes, cavern
pools) until there are twenty fish in store; fish can be eaten or cooked. A millstone
(`b`, `M`) grinds hollowgrain or goldoat into flour, which the kitchen bakes into three
meals rather than two.

**Positions.** The nobles screen (`n`) appoints a manager, bookkeeper, broker, chief medical
dwarf, militia commander and sheriff: `1`-`6` opens a list of the adults, best suited first,
each marked a good, fair or poor fit with the qualities behind it (and any post they already
hold, which they'd give up); Enter appoints, the top row leaves the post vacant, Esc cancels.
How well the holder fits sets how well the post is done:

| Post | Suits | Good / fair / poor fit |
|---|---|---|
| Manager | diligent, sociable | work orders seen to twice a day / once a day / once, one job per order at a time |
| Bookkeeper | diligent, calm | stock counted (orders seen to) 8 / 4 / 2 times a day |
| Broker | sociable, even-tempered | the merchants' tenth markup haggled to almost nothing / about half / barely cut |
| Chief medical dwarf | Medicine skill, diligent | treatment 40% / 25% / 10% faster |
| Militia commander | fighting skill, sociable | training 50% / 25% / 12% faster |
| Sheriff | fighting skill, even-tempered | culprits never / sometimes / often slip away; hot-tempered sheriffs beat harder |

Work orders only run with a manager. As the fortress grows rich and populous (☼25,000 and 20 dwarves; ☼60,000 and 30;
☼120,000 and 40) the civilization makes one dwarf baron, then count, then duke. Titled
nobles expect grand bedrooms and now and then demand a piece of furniture of a given
material in their bedroom or office; meeting it within half a year pleases them, ignoring it
does not.

**Megabeasts.** The dragons, titans, hydras, rocs and colossi of your world's history live in
lairs on the world map. Once the fortress has stood a year, one whose lair is near may come
for it in any season (more likely the richer you are). It hunts every dwarf, batters doors,
never flees, and leaves after ten days if it lives; slaying it is written into the legends. A dragon (`D`) breathes fire down a clear line at whoever it hunts, scorching everyone in
the way and setting trees, grass and wooden things alight; slay it and it leaves its skull, a
treasure, and the hoard it carried.
Once you've broken into the caverns, a cave horror may crawl up out of them: a one-of-a-kind
creature with its own name and look ("a bloated spider of crystal with many eyes") that hunts
until it's killed. Skulkers are thieves: unseen, they make for the most valuable thing lying
about, grab it and run for the edge of the map. A dwarf who comes within five tiles spots
them, and once spotted they flee and can be fought; killing one drops what it stole.

**Justice.** Punches and wrecked furniture in a tantrum, and the dwarf blamed for an unmet
mandate, go on the fortress's record of crimes, shown on the nobles screen. Appoint a sheriff
(`n`, `6`) and they seek out each culprit and beat them, closing every charge against them;
victims are glad to see justice done. With a jail zone (`i`, `j`) the sheriff sentences them
to three days a charge (at most ten) inside instead, where they do no work.

**Work orders.** `j` lists standing orders: each keeps a number of something in stock
(beds, barrels, meals, blocks...) by queuing jobs at the least busy workshop that can do
them, a few at a time. `n` adds one, Left/Right change the number, `x` deletes. In look
mode, `f` forbids an item (nobody hauls, uses or eats it) and `F` forbids everything on the
tile; `d` and `D` mark things for the garbage dump. `s` on a stockpile opens its settings:
untick kinds of item (within its category) or materials it shouldn't take. `w` on a
workshop (query it with `q`) picks who may work there.

**Industry.** Farming with seeds, brewing, cooking, butchery, charcoal, ore smelting,
steel (with limestone flux), a forge for tools, weapons and armor, stone and bone crafts,
gem cutting, furniture, barrels and bins that store goods. Multi-input recipes, repeating
work orders, item quality from skill. The butcher leaves hides; a tanner's shop (`b`, `T`)
turns them into leather, and the leather works (`b`, `L`) makes leather armor, caps and
crafts (Leatherworking labor) and leather shoes. A loom (`b`, `W`) weaves thread into cloth
and the clothier's shop (`b`, `K`) sews shirts and trousers (Weaving and tailoring labor).
Every dwarf wears a shirt, trousers and shoes, which wear out over about a year; rags
embarrass them, and they change into fresh clothes from stock when theirs are worn. The
wood furnace also burns wood to ash, the butcher leaves tallow, and the soap maker's
workshop (`b`, `P`, Soapmaking labor) turns the two into soap; doctors who clean wounds
with soap clear infection outright.

**Dwarves.** Thirst, hunger, sleep, 20 labors and skills, personalities (anxiety,
sociability, temper, diligence, artistry), favorite materials and foods, friendships and
grudges, thoughts and stress, tantrums that punch dwarves and topple furniture, and
melancholy when misery drags on.

**Rooms & nobles.** Dug, enclosed rooms are valued by size, furniture and walls; beds are
assigned to owners (nobles first) and dining at tables beats eating on the floor. An
expedition leader, later an elected mayor, demands good quarters and issues production
mandates.

**Strange moods.** A dwarf may be seized by a fey mood, claim a workshop, demand
materials and produce a named artifact, or go mad if thwarted.

**Danger.** Wildlife, cavern dwellers and sieges. Goblins attack anyone; humans, elves and
rival dwarves come when their civilization is at war with yours, each with their own gear
(elves with wooden weapons, dwarf raiders in heavy armour), led by a general out of their
history: kill the general and the whole army breaks, and the legends remember it. Body-part wounds, bleeding,
severed limbs, limping, unconsciousness and bed rest. Draft soldiers who equip the best
gear, train, station and engage; civilians flee. Doors keep animals out; locked doors stop
everyone until goblins batter them down.

**Trade.** Build a trade depot. Elves come in spring, humans in summer and your own
civilization in autumn, if they are at peace with you. Haul goods to the depot and barter
(they want a tenth on top).

**Zones & idling.** Zones (`i`) cover meeting areas, bedrooms, dormitories, dining halls,
offices, farm plots, a garbage dump, temples, taverns (a tavern doubles as a meeting area)
and tombs. Bedrooms and offices are given to a dwarf from look
mode (`k` on the zone, then `o` picks from a list with nobles and post holders first); offices
nobody owns go by themselves to nobles and post holders without one, and a post holder with
an office works a little better (it counts toward their fit); dormitory beds are shared; dining hall seats are taken first; items
marked with `d` go to the dump. Dwarves with nothing
to do gather at the meeting area and chat there more often; without one they at least keep
out of corridors. A stroll always gives way to real work.

**Needs and focus.** Each dwarf has a few needs, shaped by personality: socializing,
drinking, crafting, fighting, learning, praying and making merry. Needs fade over time and
are met by doing those things: chatting, drinking ale, finishing workshop jobs, training or
fighting, picking up new skills. A temple zone (`i`, `t`) and a tavern zone (`i`, `v`) cover
prayer and merrymaking; a dwarf whose need has gone unmet takes a break to go there. Unmet
needs lower focus (down to 70%), met ones raise it (up to 120%); focus scales work speed and
the quality of crafted goods. The dwarf screen shows focus and each need, and the status
screen (`z`) lists what the fortress is short of.

**Life and death.** Dwarves age, and few live past 150. Close friends of age may marry,
and couples have children who take their mother's family name, play instead of working,
and come of age at twelve. The dead should be buried: build a coffin (carpenter or mason,
then `b`, `y`) and a hauler carries the body there, tomb zones (`i`, `c`) first. A dwarf left
unburied for a couple of weeks may come back as a ghost that frightens everyone it drifts
near; burying the body or carving a memorial slab (mason, then `b`, `g`) lays it to rest.

**Animals.** The expedition brings dogs, goats and a cat. Pastures (`i`, `u`) keep them in and
they breed there. From look mode: `p` assigns a pasture, `w` makes a dog the war dog of a
soldier, `b` marks an animal for the butcher, and `t` tames whatever a cage trap caught.

**Medicine.** Give someone the Medicine labor and zone a hospital over some beds. Doctors
suture bleeding wounds (thread from the craftsdwarf), clean cuts before they fester, splint
broken limbs and fit crutches (from the carpenter). Untended wounds can turn infected.

**Wells.** Channel down to water (an underground pool, or a river a few levels below) and
build a well (`b`, `q`) over the hole from stone blocks or a boulder and a bucket
(carpenter). Thirsty dwarves drink its clean water when there's no ale, which bothers them
less than drinking from a river, and doctors carry buckets of it to patients in bed so
they needn't get up.

**Floodgates.** A floodgate (`b`, `j`: blocks or a boulder plus a mechanism) is built shut and
holds back water and walkers alike. Link a lever to it (`k` on the lever) and pull to open or
shut it: flood a moat, fill a cistern, or seal off a leak. Doors and raised drawbridges also
stop water.

**Magma.** Below the caverns, just above the bedrock, lies a sea of magma under a basalt roof,
with a narrow shore around its islands. Magma flows like water, only slower; it kills whatever
it touches, burns items that aren't stone or metal, and sets into obsidian where it meets
water. A magma smelter (`b`, `E`) or magma forge (`b`, `F`) built with its middle tile over
magma (open space or a floor above it) needs no fuel.

**Heroes.** Great deeds earn a dwarf a name to be remembered by: slay a dragon and become
Kelda the Dragonslayer; a megabeast, its bane (Titanbane, Hydrabane...); a cave horror,
Horrorbane; cut down an attacking general, Warbreaker; ten kills make them Grim and
twenty-five Red-Handed; making an artifact, Inspired; three arm-wrestling wins, Ironarm; two
drinking contests, Deepcup. A grander deed replaces a lesser name, never the reverse. Heroes
go by their epithet in combat, their deed goes into the legends, everyone admires them, and
when one dies the whole fortress mourns.

**Cave-ins.** Rock holds itself up only so far: dig out a spot five tiles from every wall,
pillar or undug rock (a hall eleven wide) and the ceiling comes down in rubble on whoever is
beneath. At nine wide the ceiling creaks in warning. Leave a pillar of rock, or build a wall,
in big halls. Natural caverns stand however wide they are.

**The chronicle.** At each new year the game pauses on a page telling the year that ended:
how many souls, migrants and births, the festivals kept, the wars, the foes slain, fires and
crimes, the great deeds from the legends, and the dead mourned, under a title taken from the
biggest event (The Year of the Dragon, The Year of War with the..., The Year of Sorrow, The
Year of Burning Leaves, The Year of Quiet Stone...). `c` reads it again; ←/→ turns the pages.

**Visitors.** With a tavern zone, a guest may come each season but winter. Bards (dwarves,
or humans and elves at peace with you) sing in the tavern several times a day, songs made from
the legends, your fortress's own most of all ("A Song for Kelda the Dragonslayer"): listeners
enjoy them, and a hero who hears their own deeds sung is proud. Wandering dwarf warriors rest a
couple of days and may ask to stay, joining as seasoned fighters known as the Wanderer.

**Ancient ruins.** Somewhere deep in the rock of every site lies a sealed hall of a vanished
people, its walls carved with scenes from the world's oldest legends. Mine into it and it gives
up its treasure (gold, gems and one artifact of legend), and usually wakes the warden sealed in
with them, a one-of-a-kind horror. Older saves get a hall of their own when loaded.

**Expeditions.** On the military screen, `e` sends the selected squad off the map: to explore
a ruined site, raid an enemy's town or fortress, or hunt a megabeast in its lair. The list
shows how many days each takes and how deadly it is against the squad's strength (fighting
skill, weapons, armour). They come home days later in triumph, with loot (an artifact, now
and then, from ruins) and a new name for whoever led them (Delver, the Bold, Dragonslayer,
a beast slain in the legends), or beaten, and not always all of them.

**Drama.** Cheerful dwarves play pranks (laughed off, or resented by the hot-tempered); two
dwarves pining for the same unwed one grow jealous of each other; bitter rivals trade insults,
and when one of them has a temper, a challenge to a duel at dawn. Duels are fought to a
yield: the winner is proud, the loser bloodied, and half the time they shake hands and the
grudge is over. Three duels won make a dwarf Unbeaten. The status screen's Gossip tab (`z`,
`5`) keeps it all.

**Pets.** A cat may choose a dwarf for its own, and a dwarf may take in a dog; pets get a name
("Pebble the cat"), follow their dwarf about, cheer them up, and get up to mischief that ends
up in the gossip (a rat left on a pillow, a sausage stolen from the kitchen). A dwarf grieves
a pet that dies, and a pet whose dwarf has died waits by the door.

**Legendary catches.** The site's waters hold one great fish (a great pike, a bog eel in a
swamp, a sea serpent by the ocean) with a name like Old Grimjaw. Now and then it takes a
fisher's bait and breaks the line; each escape makes it a little easier to land, and skill
helps. The dwarf who finally lands it becomes a Pike-Taker (or Eel-, Serpent-), the catch is a
masterwork, and everyone comes to marvel.

**Festivals.** Three days a year the fortress downs tools for half a day: the harvest feast
in mid-autumn, the midwinter fire, and founding day each spring from the second year. Everyone
gathers in the meeting hall or tavern (else a dining hall, else by the wagon), dances, drinks
from the stores and challenges each other at arm-wrestling (the stronger arm usually wins);
the dwarf who downs the most mugs wins the drinking contest. Everyone who came remembers it
fondly and has made merry. The top bar shows a festival while it lasts.

**Winter.** Except in deserts, swamps and by the sea, winter freezes the water under the
open sky: ice holds still, can be walked across (by wildlife too) and can't be fished, but
dwarves can still break it to drink. Spring thaws it, and anyone out on the ice scrambles
ashore. Each day brings clear skies, rain or (in a cold winter) snow, more or less often by
climate; snow lies on open ground until spring, the top bar shows the weather, and dwarves
caught out in it grumble a little. Fire catches in trees, shrubs, grass and anything wooden,
spreads (up and down slopes too), burns whoever stands in it and the items lying there, and
goes out in water or rain; flowing magma and lightning start it. Trees follow the year: blossom and pale buds in spring, yellow, orange and red in autumn with leaves lying under them, bare branches in winter; most trees in cold places are evergreen.

**Aquifers.** Many wet sites (the embark screen says which) have water-bearing rock two to four
levels down, shown with a blue background. A light aquifer only dampens floors dug out of it;
a heavy one floods anything dug beside it. Constructed walls hold it back, and an aquifer makes
a fine source for a well.

**Squads.** The military screen (`m`) forms named squads, gives them uniforms (weapon type,
armor) and orders: train, hold a station, or kill a chosen creature. Soldiers train in the
barracks zone and learn faster sparring together. Crossbows and bolts come from the
carpenter; marksdwarves shoot from up to ten tiles, through fortifications.

**Traps & levers.** A mechanic makes mechanisms from stone. Cage traps catch whatever
steps on them, weapon traps cut it up, stone-fall traps drop a boulder. Levers (`q`, then `k`
to link, `p` to pull) lock doors and raise drawbridges that nothing can get past.

**Burrows & traffic.** `w` paints burrows and assigns dwarves who must stay inside them.
Mark one as the alert burrow and raise the civilian alert from the military screen (`m`,
`c`) to pull every civilian to safety. `o` paints high, low or restricted traffic to steer
where dwarves walk.

**Labors.** `y` shows every dwarf against every labor, with skill levels. Toggle cells or
whole columns, apply named work details (or save your own), and mark specialists who never
haul. A dwarf can be a miner or a woodcutter, not both, since each needs its own tool.

**Looking around.** `k` inspects anything at the cursor: terrain, zone, building, every
dwarf, creature and item on the tile (Tab cycles, Enter opens a dwarf or building).
`z` is the fortress status screen: supplies and how long they'll last, population by
profession, what everyone is doing, happiness, full stocks, health and each dwarf's mood.

**Title, saves and loading.** The game opens on a title menu over a living scene (a mountain under a turning sky, the fortress dug into it, the magma sea below): New world, Continue, Load
game. World generation, embarking, saving and loading show a loading screen with progress.
Each fortress has its own save in `saves/` (F5 saves, F9 reloads it), plus an autosave
every season. `Q` offers save and quit, or save and return to the title.

## Tests

```
cargo test --release
cargo test --release one_year_timing -- --ignored --nocapture
cargo test --release profile_year -- --ignored --nocapture
```

## Layout

| File | Role |
| --- | --- |
| `overworld.rs` | world map, civilizations, history, legends data |
| `world.rs` | fortress tiles, movement rules, reveal, embark generation |
| `game.rs` | core simulation: jobs, needs, fluids, farms, hauling, player commands |
| `game/combat.rs` | creatures, fighting, wildlife, sieges |
| `game/psyche.rs` | relationships, rooms, nobles, mandates, moods, insanity |
| `game/trade.rs` | caravans, depot, bartering |
| `dwarf.rs`, `body.rs`, `creature.rs` | units |
| `buildings.rs`, `items.rs`, `material.rs` | data definitions and recipes |
| `path.rs` | A* and nearest-goal Dijkstra search |
| `app.rs`, `ui.rs` | input modes and rendering |
