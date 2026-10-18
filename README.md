# DwarfClone

A Dwarf Fortress-style colony simulator for the terminal, written in Rust with ratatui.
You don't control dwarves directly: you pick a site in a generated world, designate work,
lay out zones, queue orders and draft a militia, and the dwarves get on with it according
to their labors, needs, personalities and moods.

## Run

```
cargo run --release            # title screen; new worlds get a random seed
cargo run --release -- 12345   # the first new world uses this seed
cargo run --release -- --autoplay --runs 10 --years 5   # let the overseer play 10 forts
cargo run --release -- --autoplay --runs 25 --years 3 --seed 5000 --story --out after.json
cargo run --release -- --compare batches/baseline-v0.2.0.json after.json
```

A batch ends with a report on why forts fall: how many were still standing at the end of each
year, deaths by cause and by killer (a dwarf who bleeds out is put down to whoever wounded
them), where dwarves died (in a siege, outdoors, in the shelter, as soldiers), and every siege by
its order (first, second, later) with its size, losses on both sides, whether the shelter was
called and the bridge raised, and how it ended. `--story` prints each fort's timeline
(arrivals, sieges, monsters, deaths, milestones, ambitions); `--out` saves the batch, and
`--compare A B` sets two saved batches side by side, down to the forts whose fate changed.

**Autoplay.** The overseer is an AI that plays a fortress with the same commands you have:
it digs in, builds workshops and halls, keeps stocks with work orders, fills posts, sets
labors, raises and arms a squad, hangs a gate with a drawbridge on a lever and a corridor of
cage traps, shelters everyone when an army comes, mines, trades, and (by style) engraves,
raises a monument, sinks shafts or sends expeditions. As the fort grows it walls its fields and
raises towers (see **Walls and towers** below). Styles: Steady, Builder, Delver,
Warlord and Reckless. It works through *ambitions*, a season or so at a time, chosen by its
style's leanings and the fortress's troubles (hungry forts provision, besieged ones fortify and
muster): Provision (fields, fishing, hunting), Fortify (stone-fall traps, a walled yard round
the stair), Muster (more soldiers, a second squad, weapons and armour), Prosper (crafts, gems,
ore, a monument), Delve (down the shaft), Expand (new halls, then whole new levels below),
Revel (taverns, temples, engravings) and Venture (expeditions). Each is announced, shown in the
top bar, and listed in batch reports. `B` opens its brain: twelve senses (its nature, hunger,
thirst, a recent siege, crowding, unrest, riches, a monster near the halls, a strong idle
squad, being unready to march, weariness of the last ambition, and your nudges) wired into the
nine ambitions, green links pulling and red pushing, each ambition sized by its odds of being
next. Pick one and nudge it with ←→ (−3..+3) to provoke the overseer into battle or coax it
into building, or press `f` to make it take that ambition up now.

Each overseer is someone: a name and two traits on top of its style. Cautious ones fortify,
keep more under arms and bar the gate early; bold ones send expeditions and bar it late; vain
ones raise grand halls and monuments; pious and cheerful ones dig temples and taverns; greedy
ones chase riches and delve; sentimental ones keep their dwarves close; stubborn ones stick to
what they set out to do and are slow to take a hint. The traits are a sense in its brain ("Its
temper"), sway its layout, and colour its notes; `A` shows who it is and `r` there rolls another.
It also reads the announcements: whatever the fortress is short of (stone, wood, blocks,
mechanisms, drink, food, a workshop, a manager, what a dwarf in a fey mood demands), it notices
sooner or later by its style and traits and sees to it, and says so in its notes. It works in
order: each shortage is followed down to what it waits on (mechanisms need stone and a
mechanic's workshop, the workshop needs stone, stone needs a pick...), it sees to the first
things first and parks the rest ("short of mechanisms: first stone, then a mechanic's
workshop"), the thirsty and the moody before comforts. A shortage that waits on itself (no
stone without a pick, no pick without a forge, no forge without stone) is bought from the next
caravan if one sells a link of it, or given up for a season. Each worry is noted once and again
when its plan changes, and its plan stands under the notes.
While it plays, the announcements share the bottom of the screen with what the overseer has
been doing: each note it made and how many days ago.

Each overseer has a taste in halls, and its fortress follows a plan of halls first: a great
hall with rooms opening off both sides and a cross hall at its end; mirrored wings on cross
halls, every room with its twin; rooms round a pillared atrium; a grid of lanes with rooms back
to back between them; or a round hub with spokes. Rooms open onto their hall through a door.
A tidy Steady overseer leans to the great hall and the grid, a
Builder to the atrium and the wings, a Warlord to the grid, a Delver to the warren and spokes;
a Reckless one sprawls. The sprawl and the warren have no plan at all: odd rooms on tunnels,
or round, crossed and crooked rooms on winding passages. The dead get buried: when a dwarf
lies unburied the overseer orders a tomb and coffins.
Rooms come as plain halls, pillared halls, round halls, crosses and L-shapes, are only cut where
solid rock surrounds them, and always open behind the gate; when a level is full, a stair
inside the gate leads down to the next. The *Architect* ambition raises a grand work: a
pillared great hall to feast in, a rotunda with the monument at its heart, a cross-shaped
temple, or a long gallery with every wall carved.

The inside of the fort grows up with it (`game/architect.rs`, `game/maturity.rs`). A fort is
*founded*, then *settled* (the essentials dug and fitted), *established* (a dozen dwarves,
some wealth, a season of peace) and in time *comes of age*, and the overseer notes each step.
Once settled, every dwarf gets a bedroom of their own after the first dormitory: a cell in a
row off a hall, or a small room off the tunnels when the rows are used up, with a door, a bed,
and a cabinet and a chest as the carpenters make them. Couples share (a second bed goes in),
children sleep beside their parents, and once everyone has a room (and two are spare) the
dormitory is emptied and becomes a library or a parlour ("Retired the old dormitory: every
dwarf has a room of their own now."). Nobles get suites, walled into a bedchamber, an office
and a dining room with doors between. Gathering rooms follow as the fort comes up: a parlour,
a meeting hall, a tavern with a bar counter, its drink kept behind it and a stage where
visiting bards perform, a great dining hall with a chair either side of every table and statues
along the walls, a temple with an altar and statues, a barracks with targets and kit cabinets
round a clear training floor, a library of desks and cabinets; the halls are smoothed, then
engraved. Once established it rebuilds its rough beginnings: an industry quarter of halls, one
per chain of workshops (the Kitchens, the Woodworks, the Stoneworks, the Forge Hall, the
Weavers' Hall) with the stockpile they use beside them; the makeshift workshops by the wagon
come down once their replacement stands built, never while someone works at them (their queue
moves over), the wood and stone piles in the open are cleared, and a hospital wing takes over
from the first ward. Where there's no rock left it makes over a spare storeroom instead.
Taste shows: a vain overseer wants big halls, leaves pillars standing in them and fills them
with statues, and engraves everything; a cautious one keeps rooms small and plain; a pious one
puts up a temple early and a great temple later; Builders are the most lavish and the Reckless
keep their dormitories and hardly bother. Rooms have names in the look panel ("Urist and
Kadol's bedroom", "the mayor's bedchamber", "the Great Dining Hall", "the Copper Tankard
(tavern)", "the Woodworks"), and the director's camera visits a hall newly finished.
`SEED=5003 STYLE=builder DAYS=672 cargo test --release show_interior -- --ignored --nocapture`
draws a fort's levels and lists its rooms and what the overseer said of them.

Press `A` in a fortress to hand it over (or take it back), or pick
*Watch a fort* on the title screen. From the command line, `--autoplay` plays many forts side
by side and reports how each ended (`--runs`, `--years`, `--style`, `--seed`). Most end in
ruin; that's the fun of it.

**The arena.** *Arena* on the title screen sets up a battle: two sides, each any species (or
dwarves with a fighting skill from Dabbling up), how many, their training, a weapon (axe, sword,
spear, hammer, pick or crossbow, with bolts) and armour (helm, mail, shield) and what each is
made of, and a field: open ground, a corridor with a door (beasts must batter it down), a
cavern, a bridge over a chasm (a heavy blow can send someone over the edge) or a walled yard.
Enter plays the bout on its own little map with the same blows, blood, severed parts and
finishing blows as a siege; nobody eats, sleeps or works. Space pauses, `+`/`-` set the speed,
`.` steps a tick, Tab follows a fighter (their kit, wounds and kills beside the map; `f` lets
the camera follow the blood instead), `v` shows the whole combat log, `e` skips to the end. At
the end a summary tells who won, first blood, the fighter of the bout, and every fighter's
wounds, severed parts and kills. `r` reruns a matchup headless 10–500 times (`n` picks how many)
for win rates, with a table of every rerun so far; `s` saves the matchup to
`saves/<name>.arena.json` and `l` loads one (or one of ten presets: `p` steps through them,
`?` makes one up). `b` picks a winner before the bout, and the screen keeps your record.
The overseer uses the arena too: when the scouts report an army, it war-games the fight, its
soldiers as they are kitted against that many of the foe armed as such armies come, a dozen
bouts on open ground. The odds are announced (and go in the year's chronicle); grim odds
send everyone inside a day before the army comes, and after the siege the war-game is set
against what really happened.

**The director.** Leave the keys and mouse alone for 20 seconds while the overseer plays and
the camera tours the fort like a screensaver: to a fight (staying with the fighter until it
ends), a siege and the gate it comes for, a beast of legend or a monster near the walls, a
berserk dwarf, a rescue or first aid, a strange mood, migrants or a caravan arriving, a
workshop or wall going up, a wedding (the couple) or a victory feast (the fighters), a
festival, the tavern, and in quiet times dwarves at the forge, the
mining face or the fields, or a wolf pack among the trees. It pans to a subject nearby and cuts
to one far off or on another level, stays about nine seconds on each calm one, doesn't show the
same thing twice running, and says what it's watching above the map. A fight cuts in on
anything calmer. It only moves the camera; big-moment cards come first. Any key or mouse
movement gives control back (the key does what it always does).

Use Windows Terminal (or any truecolor + Unicode terminal) at 130x40 or larger.
Press `?` in game for keys.

**Word gets around.** A dwarf who goes mad because the fortress never had what their fey mood
demanded (gems, metal, good stone, wood, bone) is talked of. The merchants carry the talk home
when they pack up; failing that, the tappers of the deep roads, who knock news from hold to hold
through the rock, get it to the mountainhome within the season. Then dwarves there who love
that thing stay away, and the migrants say so ("word that there are no gems here kept 2 at
home"). When the fortress finally has some, better word goes out and they come again.

**Big moments.** A siege at the gates, a dragon or a beast of legend, something from the
caverns, the dead rising, traders at the depot, a hero struck down, a cave-in, a ghost, an
artifact, a new noble, a siege broken or its leader slain: the game pauses, turns the camera to
where it happened and tells it in a card over the map (Enter goes on, Space goes on and
unpauses). When the overseer plays, the card goes by on its own after a few seconds.

**Side panes.** The status (`z`), brain (`B`), reports (`r`), chronicle (`c`) and legends (`L`)
screens open beside the map, which stays live in a narrower view; `\` switches between the side
pane and full screen.

**Performance.** `F3` shows frames per second, how long each frame spends simulating and
drawing, ticks and path searches a second, the slowest tick, and how the simulation's time
splits between water, the overseer, dwarves and creatures. For batches, `TRIAL_PROF=1` prints
the same every ten days of each fort, `SPIKE_MS=n` reports any tick slower than that, and
`--threads n` sets how many forts play at once.

**Game over.** When the last dwarf dies, the fortress has fallen: a page tells how long it
stood, who lived there, its sieges, heroes and monument, how they died and who fell last, with
the chronicle, legends and reports a key away, or a look upon the ruins. Returning to the title
keeps the ruin saved.

**Mouse.** Click any key entry in the side panel or a menu to use it, and rows in lists
(appointments, room owners, expeditions, autoplay, the title menu) to pick them. On the map,
click to move the cursor (click again to look), drag to pan, scroll to change level, and
right-click to go back. In digging, zone, stockpile, traffic, burrow and construction modes,
drag a box; a click marks a single tile (with the keys, Enter sets each corner). Ctrl+Z
undoes the last designation, and designated rock shows the job on the wall (▒ dig, < > X
stairs, ▼ channel). Hovering over the map shows what's there.

## Features

**World creation.** New world opens a settings screen: preset, seed, world size (128×56,
192×80 or 288×120), history length, number of civilizations, savagery and minerals. Then you
watch the world being made, in about fifty seconds: an empty sea; land rising out of it as the
water falls away, its coasts spreading; mountain ranges thrust up and whitening at the peaks,
volcanoes erupting in lava and smoke; the erosion phase's trial rivers running down the heights
and wearing them; the real rivers growing from their springs to the sea, tributaries joining,
lakes filling their basins; rain and warmth bringing the biomes in (snow on the cold heights,
forests greening from the wettest land outward, deserts last), a wildfire or two sweeping dry
grass and greening back; and then history, with the year counting up, hamlets, towns, holds
and lairs appearing in the year they were founded, battles and raids flashing between places,
ruins where they fell. A caption names each stage. It is the generator's own record played
back, not a second simulation: the same seed makes the same world, watched or not. Space or
Enter skips to the finished world; accept it or roll another.

**World & history.** The world is made the way Dwarf Fortress makes its own
(`docs/worldgen.md`): not by placing biomes but by generating fields over every world
cell and reading the biomes off them. Elevation comes from midpoint displacement with
ranges lifted along ridge lines; temperature from latitude and height; rainfall from a
fractal plus the prevailing winds, which drop their water on the windward slopes and leave
rain shadows past the crests; drainage, savagery and good and evil are fractals of their
own; vegetation follows rain and warmth; salt lies in the sea and a cell or two inland.
A threshold table turns those into mountains, hills, forest, taiga, jungle, grassland,
savanna, shrubland, desert, badlands, tundra, glacier, swamp and marsh (salt marsh on the
coast), so borders wander like coastlines and no desert lies beside a jungle without the
grassland between. An erosion phase then runs temporary rivers from the foot of every
range, wearing the heights and cutting channels to the sea, before the real rivers run:
every cell drains to the sea or a lake (the deepest hollows fill into lakes; in dry
basins they keep their water), flow gathers downstream, and the bigger streams are named
rivers with their tributaries while the smaller are brooks, too small for the world map
but there on the land. Connected stretches of one broad kind of land are named regions
(one forest can be taiga in its cold north and jungle in its warm south), each with the
wildlife and plants that live there: animals have habitats and geographic ranges, so the
same kind of land in two corners of the world holds different beasts. Cave rivers run
long courses through the deep rock. The dwarven, human, elven and goblin
civilizations settle the land the fields made. Two centuries of history are simulated before you arrive: settlements are
founded, wars declared, sites sacked or conquered, rulers succeed one another, megabeasts
ravage towns and heroes slay them. Seas, mountain ranges, forests and plains carry names. Big worlds scroll, with a minimap.
Browse it all in the legends viewer (`L`). You choose
your embark site; the biome shapes the map (relief, forest, lakes, savage wildlife) and
nearby goblins, and any civilization at war with yours in the world's history, decide how
often sieges come.

**The world map.** One character a cell, tight like the old game's atlas: each region is a
solid texture of one glyph family in two or three shades of its colour (green `∩` `n` hills,
`♣` forests, `"` grass, grey and white `▲` `^` mountains with snow on the high peaks, `↑`
taiga, `♠` jungle, `%` swamp, sandy `~` desert, icy `░` `▒` glacier, rust `∩` badlands, khaki `τ` shrubland, green-blue `"` marsh, lighter `≈` lakes), the
sea a uniform blue `≈`, and no blank-looking dots or commas anywhere on land. Rivers are thin
light-blue lines (`─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`) from cell to cell; sites stand out as small bright
marks (`Ω` dwarves, `#` towns, `+` hamlets, `♠` elves, `Ψ` goblins, `&` lairs, `×` ruins) and
a few red `▲` volcanoes smoulder. A key in each mark's own colour runs under the map. The
climate runs from ice caps at both poles through taiga and tundra to jungle, savanna, desert
and badlands in the warm middle.

Rivers respect the land: each gathers where enough rain has run together and only ever
runs downhill, to the sea, into a lake, or into the river it feeds, carrying more water the
further it goes (the great rivers are drawn a deeper blue, and the world info names the
river through a cell). A site has a river exactly when the map draws one through its cell,
an underground river exactly when a cave river runs beneath it, and a volcano brings its
crater.

**The land you embark on.** The local map is built from the world, the way the old game
builds it: every world cell is 16×16 region tiles (three map columns each), worked out on
demand from the world's fields, and the fortress map is drawn from the tiles under it. The
tiles carry the fields smoothly across cell borders plus detail of their own (rolling on
the plains, ridged in the mountains, stepped where the rock is hard), the sea and lakes
with shores that wander across the cells, the courses of the world's rivers and brooks,
a soil depth and the geology of their province (sandstone and limestone beds over granite,
marble over granite, basalt flows, or granite all the way down). The map takes one height
scale for all its land, runs its surface through the tiles (smooth between tiles of about
one height, a cliff of hard rock where they jump), lays its sea, lakes, rivers and brooks
where the tiles have them, its soil as deep as the tiles say, its trees as thick as their
vegetation and its stone as their geology, and then the processes below finish it. So two
fortresses embarked side by side meet on the same land at their shared edge. Each biome has its own
ground and growth: sand dunes and cacti in the desert, red clay and scrub in the badlands, bare
rock on the mountain heights, snow that never melts on a glacier, short pale grass on the tundra,
dark grass under the pines of the taiga, tall golden grass and acacias on the savanna, dense
palms and undergrowth in the jungle, reeds in the swamp, sickly grass and twisted trees where
evil lies. The rock beneath differs too: granite and marble in the mountains and the cold north,
sandstone and limestone in the warm lowlands, with thick soil on the plains and almost none on
the heights. And the neighbours reach in: the map stands on its world cell at the top left and
runs east and south into the cells beside it, so a coast shows the sea along that side,
mountains next door lift the land toward them, and a big embark spanning several cells
blends their biomes along ragged borders. Growing the map (`M`) carries on into the
neighbours the same way.

The land is finished by processes, one after another (`src/terrain_gen/`). Dunes rise in the
desert, a volcano's cone, mesas and sharp peaks where the site has them. The rivers, along
the region tiles' courses, cut their valleys first: broad in soft ground, a gorge in hard
rock, their water never rising along the way. (A map with no world behind it, or from a save
made before the region tiles, is shaped the old way: the world cells' heights set the level
and the tilt, and each cell's biome picks its landform.) Then the rain wears the land:
droplets carve gullies and lay alluvium in the flats, and slopes too steep for their rock
crumble to scree, so cliffs stand only where the rock is hard. Water gathers where the land
says: streams where the run-off collects, ponds and lakes in the hollows (fewer in dry
country, where the water stands below the brim), the sea with its beaches. Forests grow
thickest where the ground is wet and thin on ridges, with clearings and meadows; reeds edge
still water; cliffs and crags are bare rock; snow lies on cold heights. Underneath, tunnels and
caves wind through the upper rock, some opening on a cliff face, lava tubes run from a volcano,
and the caverns, aquifer (likelier under wet lowland) and magma sea lie below as before. Cliffs
or not, every piece of land can be walked to (a gully is cut down a scarp, a ford through a
river where nothing else will do), and the wagon stops on dry, level, reachable ground with
room round it, the caves kept clear of where the fortress will grow.

Wildlife follows the land too: the grazers, pack hunters and rare beasts that come to the
fortress are its region's own (the world info and the legends list each region's wildlife
and plants), and the savagery field makes a site wilder or tamer. Where a region lacks one
of them the land's usual one comes: deer, wolves and bears in the forests and hills; elk,
wolves and bears in the taiga; elk and polar bears on the tundra and glaciers; ibex in the
mountains and badlands; camels, giant scorpions and vipers in the desert; antelope and lions
on the savanna and shrubland; jaguars and vipers in the jungle; vipers and alligators in the
swamp and marsh. Where evil lies on the land, walking husks come in their place half the time.

The wild live somewhere. At embark the land gets a grazing ground, one or two dens of its
pack hunters and a lair of its rare beast, out towards the map's edge in woods or on high
ground. Each holds so many beasts; a party comes out of one with some at home, and when it
leaves it walks back there (a dwarf who sees it go in, or a squad hunting it, finds the
place: `Ω` on the map, and the look panel says how many are at home). Those the fort kills
are gone. When the last of a den dies, the woods there fall silent ("The woods to the north
have gone quiet") for a year, and it goes in the chronicle; otherwise each lair grows back by
one a season, and in spring the young are seen about a known den. A den that keeps sending
packs is a target: with no beasts about the fort, the overseer marches a squad on it, and
the wolves at home come out to meet them. Empty the grazing grounds and the hunters grow
hungry and bolder.

Beasts with sense weigh their chances. Three dwarves together make a wolf back off (each
swing it may lose its nerve, the likelier the more friends stand by); flames scare the wild,
and a beast once burned keeps well clear of fire. When something takes up a trail, the birds
go quiet near the dwarves under the sky ("The birds have gone quiet near the woodcutter
Urist"), they grow uneasy, and an overseer takes it as warning and keeps the work indoors.

**Sites and maps.** Every site has its own features, shown on the world map before you
embark: a river with fords, a chasm crossed by natural bridges, a volcano with a magma
crater, terraced mesas, sharp peaks, an oasis, a glacier, a bog of shallow pools, a coast, tors
of bare rock, an underground river or a hidden gem grotto. Pick the map size there too: `s`
for Small, Medium or Large, `[` `]` for width and `{` `}` for height (16 at a time), `d` for
depth. In the fortress, `M` grows the map north, east, south or west (16 tiles a press): the
new land continues the old, the fortress and everything in it stays where it was, and the
status screen shows the land's features and size.

**Embarking.** Enter on a site doesn't embark straight away: a card sums the site up (biome
and region, features, aquifer, wildlife, nearby beasts, the nearest neighbours and which of
them are hostile) and offers three choices, so a slipped key can't commit you:

- **Play now** (`p`): the usual seven (two miners, a woodcutter, a carpenter, a mason, a
  farmer and a brewer, each with a second labor) and the usual wagon: 30 food, 40 drink, 6
  logs, a spare axe and pick, 35 seeds of five crops, 2 barrels, 2 dogs, 2 goats and a cat.
- **Prepare carefully** (`c`): spend a budget of 400 points on the expedition. Each
  founder's main and second labor can be changed (Space) and their skill raised or lowered
  (←→, Tab for main or second): a skill costs 5, 15, 30, 50 or 75 points in all at Novice,
  Adequate, Competent, Skilled or Proficient. The wagon's food, drink, seeds and granite
  blocks come 5 at a time for 5 points; logs and barrels cost 3 each, axes and picks 10,
  threadreed cloth 3 and thread 2; dogs 10, goats 12 and cats 8 each. The standard party
  costs 341, leaving 59 to spend; going over the budget won't embark. `r` starts over, Enter
  embarks with that loadout, Esc goes back to the card.
- **Back** (Esc): to the map.

The default site (and the overseer's own embarks, which always take the standard party)
prefers open land near your kin, with a river and without an aquifer.

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
gem cutting, furniture (cabinets `b x` and chests `b H` from the carpenter, statues `b S` from
the mason; a tavern stage `b A` from a stone), barrels and bins that store goods. Multi-input recipes, repeating
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

**Close quarters.** Fighters close and stay engaged: one with a foe beside it strikes rather
than walks, pursuit makes straight for a free tile beside the quarry, and several attackers
spread round one target instead of piling onto a tile (a soldier fights the goblin at their
elbow rather than walking off after the one they meant to). Fights end when someone flees,
routs or falls. Creatures fight each other too, by a simple hostility rule: armies of
different peoples, armies and wild predators, predators and the grazers they now and then
hunt (the deer bolts), cave dwellers and intruders, and the walking dead and the monsters of
legend against everything living. Merchants, guests and the fort's own animals are left out
of it, and only a foe close by is noticed, so the land doesn't become a slaughterhouse.

**Brutality.** How bad a blow is depends on the weapon, the edge, the force and the size of
whoever swings it. A mangled limb may come off to a heavy edge (or be torn away by a beast
much bigger than its victim); a fatal edged blow to the head may take it off; blunt blows
shatter bones, cave in skulls and knock teeth out; a heavy edge sometimes bites through
armour; a bear, an alligator or a megabeast sends a dwarf sprawling a tile back (and
stunned), smashing any table, chair, bed or door they land in. A fighter whose weapon arm is
ruined drops what they hold. Only the notable blows are told in the reports (severings,
broken bones, teeth, devastating hits, kills, armour cut through): no more lines for every
scratch, block and deflection.

**What a fight leaves.** Blood spatters where blows land (and on the tile beyond for a heavy
one), pools where someone bled hard or died, and trails behind the wounded who walk on: red
for dwarves, goblins and beasts, yellow-green ichor for spiders and scorpions, black for the
walking dead, violet for the horrors of the deep. It dries and fades over a day or so in the
open (the rain washes it off within hours, standing water at once) but stays a season under a
roof unless a hauler with nothing else to carry scrubs it up (the cleaning job, once no
hostiles are about). Severed parts lie where they fell as items with their owner's name
("Urist's left arm", "goblin's head", "Urist's 2 teeth"), refuse to be hauled to the refuse
pile or the dump, and rot like any body; dropped weapons, armour and corpses stay until
someone takes them. Fire and dragon breath throw up smoke that drifts and thins (slowly in the
halls). The map shows it all under the units: blood as a dark stain (`≈` for a pool), smoke as
a grey haze; `k` describes it ("A pool of goblin blood", "Urist's severed left arm"). Dwarves
who see a limb or a head come off, or wade through a pool of blood, are shaken by it, less so
each time (and soldiers of some skill hardly at all: "was unmoved by the carnage"); the
director's camera visits the field of a fight while the blood is fresh. Stains and smoke are
kept in small capped maps, never on the tile, and cost nothing when there are none.

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

**Wild weather.** Rain can break into a thunderstorm, when lightning strikes far more often; a
cold winter's snow can turn to a blizzard that chills anyone caught outside; some summers bring
a drought, with no rain and shallow water drying away; and over an evil site, rain sometimes
falls the colour of blood, a terrible thing to be caught out in.

**The necromancer.** Some sites have a necromancer for a neighbour, in a tower nearby. After
the first year, now and then, they raise the dead against you: a horde of walking corpses from
the edge of the map, and every one of your own dead left unburied rises where it lies (the more
unburied dead, the more often they come). Strike a risen dwarf down and the corpse is back,
to be buried properly. Sometimes the necromancer (`N`) comes along to watch; destroy them and
all the dead fall still, for good, and whoever did it is known as Deathbane.

**Milestones.** The fortress's firsts and great numbers get a glinting banner across the map
and a place in the status screen's list: the first masterwork, artifact, child, festival,
hero, broken siege, dragon slain, vault opened, great fish landed, triumphant expedition, a
year without a death, the necromancer destroyed; twenty, fifty and a hundred dwarves; ☼10,000
up to a quarter of a million in wealth.

**Mysteries.** Small stories that unfold over days, each once per fortress. A miner who strikes
a gem may bring up the Whispering Stone; it speaks to them at night, they stop sleeping, and
then either they wake knowing the way to the sealed hall in the rock (its carved walls appear
on the map) or they're taken by a fey mood. After the first year a hooded stranger may come
asking for one of your dwarves by name and sit watching them; when the hood comes off it's a
long-lost sibling or friend come home to stay, or a goblin spy who slips away, and the next
siege is sure to come.

**Megaprojects.** `b`, `G` places a great monument (3×3) that takes thirty stone blocks, each
carried and set in place by a builder: a long job that rises as scaffolding. When the last
block is set it's dedicated to the fortress's greatest hero (or its leader) as the Statue of
whoever it is, everyone is proud, a dedication feast begins, the year is named for it, and
it's worth a fortune. Dwarves who pass it stop to admire it. (The build menu is now a
two-column list, so every building fits on screen.)

**Walls and towers.** A grown fortress walls itself in, a stretch and a storey at a time,
with stone its miners cut (from outcrops above ground first, the shortest haul), else blocks, else
timber. Once the gate's drawbridge works and the fort has people and some wealth (or has seen an
army), the overseer lays out a **curtain wall** round the stair, the fields and the workshops by
the wagon, its gate in the middle of one side between two **gate towers** (5×5, two storeys and a
platform behind fortifications); after the walls come battlements along their tops, a walkway
behind them that runs out of the gate towers' first storeys, and an arch over the gate; at last a
**drawbridge** in the gate on a lever in a gate tower, which rises when the gate below is barred.
The gate itself is never built over, so an army always has a way in to the gate below. Then
**watchtowers** at the corners (the one on the highest ground first, more with more dwarves and
every siege). Warlords, the cautious, an overseer set on fortifying, or one that lost dwarves
under the open sky start sooner; the reckless never do. On land too broken for a curtain, a lone
watchtower goes up on the high ground. In peace a soldier keeps **watch** from a watchtower's top:
a manned watchtower sees an army half a day sooner ("The watch on the North-West Watchtower
sees..."). In an alert the marksdwarves hold the gate towers' tops and **shoot down** over the
battlements at anything under the open sky below (a level of height adds a tile of range); the
towers' doors are barred when the enemy comes close, once the archers are up. A vain, pious or
greedy overseer (or a Builder, or anyone in a very rich fort) raises a **wizard's tower**, five
storeys with a walled, windowed and roofed **laboratory** at the top ("the Spire of Urist", "the
Star Tower", "the Counting Tower"), where dwarves with a need to learn go to study. A rich fort
with its walls done adds **grand works**: the Great Portico (pillars and a roof over the stair),
pillars round the monument, and a bridge over a chasm or sunken river that makes the fort walk
the long way round. `k` names what a tile is part of ("Part of the North-West
Watchtower"), the director's camera visits the watch and the works going up, and the overseer
notes each step in its own voice.

**Rot and miasma.** Corpses and carcasses left lying about rot. After five days they stink:
under the open sky the smell blows away, but underground it gathers as miasma (a purple haze)
that drifts through open halls (not past locked doors), thins slowly, and sickens every dwarf
who breathes it. After forty days only bones are left, except a citizen's body, which keeps
on stinking until it's buried. Put a refuse stockpile outside, and bury your dead.

**Festivals.** Three days a year the fortress downs tools for half a day: the harvest feast
in mid-autumn, the midwinter fire, and founding day each spring from the second year. Everyone
gathers in the meeting hall or tavern (else a dining hall, else by the wagon), dances, drinks
from the stores and challenges each other at arm-wrestling (the stronger arm usually wins);
the dwarf who downs the most mugs wins the drinking contest. Everyone who came remembers it
fondly and has made merry. The top bar shows a festival while it lasts.

**Stories with names.** A siege with blood spilt, a run of deaths to one kind of beast, or a
great hunt gets a name once it's over, from where it broke and how it went ("the Battle of
Copper Gate", "the Wolf Winter", "the Night of Red Knives"): it's announced, gets a spotlight
card, a legend, a chronicle line and can name the year. The place where it broke is named for
the deed done there ("Kolvar's Stand", "the Weeping Gate"); look shows the name, the deed and
the year, and later fights there are named after it. A newborn may be named for one of the
fallen the parents held dear, with the reason in the chronicle. Friends who marry are wed
before the fort (in the temple, the great hall or under the sky), their closest friends bring
gifts, a wedding feast follows, and a rival in love may turn up uninvited, to weep at the back
or challenge the groom to a duel at dawn. When a siege is broken, the overseer calls a victory
feast in its own voice ("We held. We always hold."), the whole fort cheers, and the dead are
toasted by name. Names and chances come from a hash of the fort and the moment, never the
game's dice (`stories.rs`).

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
carpenter; marksdwarves shoot from up to ten tiles, through fortifications, and further down
from a tower.

**Traps & levers.** A mechanic makes mechanisms from stone. Cage traps catch whatever
steps on them, weapon traps cut it up, stone-fall traps drop a boulder. Levers (`q`, then `k`
to link, `p` to pull) lock doors and raise drawbridges that nothing can get past.

**Burrows & traffic.** `w` paints burrows and assigns dwarves who must stay inside them.
Mark one as the alert burrow and raise the civilian alert from the military screen (`m`,
`c`) to pull every civilian to safety. `o` paints high, low or restricted traffic to steer
where dwarves walk. The overseer's shelter keeps the gate (stair foot, doors, bridge) outside
it, so those running in go on past the bridge; keeps a larder inside it, a small stockpile the
haulers fill with drink and meals (four drinks and two meals a dwarf) whenever there's no
alert; and walls up every other way in that reaches the sky, daily and the moment the alert
goes up. A siege sheltered through without loss is told.

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

## Releases

Pushing a version tag builds the Windows game on GitHub and publishes it as a release
(`.github/workflows/release.yml`): the exe has the C++ runtime built in and ships zipped with
`release/HOW TO PLAY.txt`.

```
git tag -a v0.2.0 -m "DwarfClone v0.2.0"
git push origin v0.2.0
```

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
| `worldgen/` | the world's land: fields and biomes, erosion and rivers, regions and wildlife, region tiles |
| `world.rs` | fortress tiles, movement rules, reveal, embark generation |
| `game.rs` | core simulation: jobs, needs, fluids, farms, hauling, player commands |
| `game/combat.rs` | creatures, fighting, wildlife, sieges |
| `game/wildlife.rs`, `game/lairs.rs`, `game/hunts.rs` | predators' sense, fire and standing together; lairs and living populations; the overseer's hunts |
| `game/melee.rs`, `game/gore.rs` | close quarters, creature feuds; blood, smoke, severed parts |
| `game/psyche.rs` | relationships, rooms, nobles, mandates, moods, insanity |
| `game/trade.rs` | caravans, depot, bartering |
| `game/overseer.rs`, `game/layout.rs`, `game/blueprint.rs` | the overseer AI: styles, room plans and layouts |
| `game/architect.rs`, `game/maturity.rs` | the fort's insides: bedrooms, suites, gathering halls, taste; maturity, the industry quarter, rebuilding |
| `game/fortify.rs`, `game/towers.rs` | the curtain wall, towers and grand works outside; the watch, tower stations, shooting down |
| `dwarf.rs`, `body.rs`, `creature.rs` | units |
| `buildings.rs`, `items.rs`, `material.rs` | data definitions and recipes |
| `path.rs` | A* and nearest-goal Dijkstra search |
| `app.rs`, `ui.rs` | input modes and rendering |
| `game/arena.rs`, `game/arena/` | the arena: matchups, fields, the bout's own loop, reruns, saved matchups |
| `game/wargame.rs` | the overseer war-games a coming siege in the arena |
| `app/arena.rs`, `ui/arena.rs` | the arena's screens: setup, the bout, summary, report, win rates |
