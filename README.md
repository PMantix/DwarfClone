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
nearby goblins decide how often sieges come.

**Fortress.** A small (64×48), medium (96×64) or large (144×96, 48 levels deep) region, picked
with `s` on the world map: soil over sedimentary and igneous rock, ore veins, gems,
and hidden cavern layers (two on deep maps) with pools, tower mushrooms and things that wake when you
break in. Designate mining, channels, stairs, ramps and tree felling. Water falls and
levels out.

**Constructions.** `b`, then `C`: plan walls, floors, stairs, ramps and fortifications
in a box, built from rough stone, blocks or logs. Take them down again to get the material
back. Floors bridge open space, fortifications stop walkers but let water through, and a
roof over surface walls makes a proper room.

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

**Work orders.** `j` lists standing orders: each keeps a number of something in stock
(beds, barrels, meals, blocks...) by queuing jobs at the least busy workshop that can do
them, a few at a time. `n` adds one, Left/Right change the number, `x` deletes. In look
mode, `f` forbids an item (nobody hauls, uses or eats it) and `F` forbids everything on the
tile; `d` and `D` mark things for the garbage dump. `s` on a stockpile opens its settings:
untick kinds of item (within its category) or materials it shouldn't take.

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

**Dwarves.** Thirst, hunger, sleep, 19 labors and skills, personalities (anxiety,
sociability, temper, diligence, artistry), favorite materials and foods, friendships and
grudges, thoughts and stress, tantrums that punch dwarves and topple furniture, and
melancholy when misery drags on.

**Rooms & nobles.** Dug, enclosed rooms are valued by size, furniture and walls; beds are
assigned to owners (nobles first) and dining at tables beats eating on the floor. An
expedition leader, later an elected mayor, demands good quarters and issues production
mandates.

**Strange moods.** A dwarf may be seized by a fey mood, claim a workshop, demand
materials and produce a named artifact, or go mad if thwarted.

**Danger.** Wildlife, cavern dwellers and goblin sieges. Body-part wounds, bleeding,
severed limbs, limping, unconsciousness and bed rest. Draft soldiers who equip the best
gear, train, station and engage; civilians flee. Doors keep animals out; locked doors stop
everyone until goblins batter them down.

**Trade.** Build a trade depot. Elves come in spring, humans in summer and your own
civilization in autumn, if they are at peace with you. Haul goods to the depot and barter
(they want a tenth on top).

**Zones & idling.** Zones (`i`) cover meeting areas, bedrooms, dormitories, dining halls,
offices, farm plots, a garbage dump, temples, taverns (a tavern doubles as a meeting area)
and tombs. Bedrooms and offices are given to a dwarf from look
mode (`k`, then `o`); dormitory beds are shared; dining hall seats are taken first; items
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

**Title, saves and loading.** The game opens on a title menu: New world, Continue, Load
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
