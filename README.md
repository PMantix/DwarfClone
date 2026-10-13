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

**Industry.** Farming with seeds, brewing, cooking, butchery, charcoal, ore smelting,
steel (with limestone flux), a forge for tools, weapons and armor, stone and bone crafts,
gem cutting, furniture, barrels and bins that store goods. Multi-input recipes, repeating
work orders, item quality from skill.

**Dwarves.** Thirst, hunger, sleep, 13 labors and skills, personalities (anxiety,
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
offices, farm plots and a garbage dump. Bedrooms and offices are given to a dwarf from look
mode (`k`, then `o`); dormitory beds are shared; dining hall seats are taken first; items
marked with `d` go to the dump. Dwarves with nothing
to do gather at the meeting area and chat there more often; without one they at least keep
out of corridors. A stroll always gives way to real work.

**Medicine.** Give someone the Medicine labor and zone a hospital over some beds. Doctors
suture bleeding wounds (thread from the craftsdwarf), clean cuts before they fester, splint
broken limbs and fit crutches (from the carpenter). Untended wounds can turn infected.

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
