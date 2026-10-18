# World generation on fields

How the world map is made, and how the fortress map is made from it. This
replaces the old scheme (one height noise with ridges, biomes picked from
height, rain and heat noise in one pass, rivers walked downhill from random
springs, the local map's relief invented per biome) with a field-based one in
the manner Tarn Adams describes for Dwarf Fortress: generate a few basic
fields per world square, derive the biomes from them, wear the land with an
erosion phase, run real rivers, group the land into named regions, and build
the local map from a finer grid of region tiles under the embark.

Code: `src/worldgen/` (new) holds the world-scale pipeline; `overworld.rs`
keeps the types, sites, civilisations and history; `terrain.rs` and
`terrain_gen/` build the local map.

## 1. Fields

One value per world cell, generated in this order. All are computed as
`f32`; elevation is kept on the old 0..255 scale (sea level 92) so the
existing thresholds and the u8 `Cell::elev` stay meaningful, the rest on
0..100. Fields live in `Overworld::fields` (one byte per cell per field,
saved compactly as base64 strings).

| field | how it's made |
|---|---|
| elevation | Midpoint displacement (diamond-square) over the world on a lattice whose coarsest step is 32 cells. The coarse points are seeded with a continent bias (low toward the world's edges, a few random uplifted masses), the displacement halves (roughness 0.55) at each level. A ridge uplift (ridged noise, only inland) adds long ranges, since pure midpoint displacement makes blobs. Then a monotone piecewise-linear remap fixes the proportions: 40% of cells below sea level (92), the land's 80th percentile at the hills line (148), its 93rd at the mountain line (170), the highest at 255. |
| temperature | Latitude first (hot at the equator, cold at both poles), plus a little fractal noise (+-8), minus a lapse with height above sea level (up to about 45 at the highest peaks). |
| rainfall | An independent midpoint-displacement fractal, 0..100, then biased by orographic effects: prevailing winds by latitude band (easterly trades in the tropics, westerlies in the middle latitudes, polar easterlies), and a moisture sweep along each row in the wind's direction: air picks moisture up over the sea, drops part of it as it climbs (wet windward slopes) and has little left past the crest (rain shadow on the lee), drying slowly inland. Final rain = 0.45 fractal + 0.55 sweep, then smoothed so no two neighbouring cells differ by more than about 15. |
| drainage | An independent midpoint-displacement fractal, 0..100 (how fast water leaves the ground: low is boggy, high is rocky and free-draining). |
| vegetation | Derived from rainfall and temperature (little in the cold and the dry, most in the warm and wet), 0..100. It's a field of its own so it can be altered: towns and hamlets log the land round them during history. The local map's tree density reads it. |
| savagery | Independent fractal, 0..100. Feeds the wildlife mix and `EmbarkParams::savagery`. |
| good/evil | Independent fractal, 0..100: below 22 is evil (the old `Cell::evil` flag, about the same share as before), above 82 good (calmer wildlife, shown on the world info). |
| salinity | 100 on ocean cells (not lakes); over land it drops 34 a cell from the nearest ocean, so only the first cell or two of coast is salty. It separates salt marsh and mangrove swamp from fresh marsh and swamp. |

Fractal fields make biome boundaries look natural: two biomes meet where a
smooth field crosses a threshold, so the border wanders like a coastline and
the order of biomes across it follows the gradient.

## 2. Biomes from the fields

`T` temperature, `R` rainfall, `D` drainage, `E` elevation, `S` salinity.
Tested top to bottom; the first rule that matches wins.

| rule | biome |
|---|---|
| E < 92 | Ocean |
| a cell the river phase filled into a lake | Lake (new) |
| E >= 170, T < 12 | Glacier |
| E >= 170 | Mountains |
| T < 8 | Glacier |
| T < 22 | Taiga if R >= 45 and T >= 14, else Tundra |
| R >= 66 and D < 33 | Swamp (mangrove swamp where S >= 66) |
| R >= 66 and D < 50 | Marsh (new; salt marsh where S >= 66) |
| E >= 148 | Badlands if R < 20 and T >= 40, else Hills |
| R < 12 | Badlands if D >= 66, else Desert |
| R < 30 | Shrubland (new) if D >= 50; else Savanna if T >= 62, else Grassland |
| R < 50 | Savanna if T >= 62, else Grassland |
| T >= 62 | Jungle |
| T < 32 | Taiga |
| otherwise | Forest |

Because rain is smoothed, desert (R < 12) can't sit beside jungle (R >= 50)
or swamp (R >= 66): grass, savanna or shrubland always lies between.

## 3. Erosion phase

Midpoint displacement leaves straight-line artefacts along its lattice and
pits everywhere. Before any river is run:

1. Mountain bases: every land cell at or above the hills line, in a seeded
   order, a few passes over.
2. From each, a temporary river: step to the lowest of the four neighbours
   if it's lower; if none is, dig the cell below the lowest neighbour and
   step there. Every cell passed over is worn a little. The path ends at the
   ocean, at the world's edge, or after 300 steps (stuck).
3. That wears the mountains down along their drainage lines and opens
   channels through rims to the sea. The elevation is remapped again
   afterwards so the proportions of sea, hills and mountains hold.

## 4. Rivers

1. Depression filling: a priority flood from the ocean and the world's
   edge gives every land cell a receiver (a 4-neighbour it drains to) and
   a spill level. Paths that can't reach the ocean are forced through the
   lowest point of their rim, so every river reaches the sea or a lake.
2. Lakes bulge out: a depression whose fill is at least 3 deep, or whose
   catchment is large, floods to its spill level and becomes Lake cells.
   In arid basins (mean catchment rain below 20) the lake is terminal: the
   water evaporates and no river leaves it (a salt lake).
3. The receiver tree has no loops by construction (the loop-erasure DF
   needs for its random-walk rivers is implicit).
4. Flow: each cell contributes its rain (`0.05 + R/100`), accumulated
   downstream in order of falling spill level. A cell with flow at or above
   the river threshold is a river; flow keeps growing downstream.
5. Tributaries and names: from each mouth, the main stem follows the
   biggest upstream branch; every other branch that reaches the river
   threshold is a tributary of the stem it joins. Each stem gets a name
   ("the River Sileth", "the Ashen Water"...) and is stored with its course,
   mouth (sea, lake or river) and the river it feeds.
6. Brooks: cells with flow between the brook and river thresholds, in land
   with some rain, carry a brook (links like a river's, not drawn on the
   world map, but present on region tiles and the local map).
7. Elevation along a river never rises downstream (the spill levels), so
   rivers run downhill by construction.

Cell links (1 north, 2 east, 4 south, 8 west) are written for rivers into
`Cell::river` (as before, so the world map, site features and the local map
keep working) and for brooks into `Cell::brook`.

## 5. Regions and populations

Connected cells of the same broad region type are grouped and named:
Ocean/Sea (by size and edge), Lake, Mountains (mountains, glacier on
mountains), Hills, Forest (forest, taiga and jungle together: one "Silvery
Forest" can be taiga in its cold north and jungle in its warm south),
Plains (grassland, savanna, shrubland), Desert (desert, badlands), Wetland
(swamp, marsh: stored as the old `Swamp` kind), Tundra, Glacier. Patches
under 3 cells stay nameless as before.

Each region gets a population: the wildlife species and the plants that
live there. Every wild species has a habitat (temperature, rain and biome
range) and a geographic range: a few seeded centres on the world with a
radius, so the same biome in two corners of the world can hold different
animals (bears in the western forests, not the eastern). A region holds the
species whose habitat matches enough of its cells and whose range reaches
it. The fortress's wildlife (grazers, pack hunters, rare beasts) is drawn
from the region's population; with no population (an old save) it falls
back to the old table by biome.

Non-local features are laid out here too: a few cave rivers run long
courses across the world through the deep rock; a site has an underground
river exactly when one passes under its cell.

## 6. Region tiles

Each world cell is 16x16 region tiles; a local tile is 1/3 of a region tile
(the fortress map has 48 tiles to a world cell), so a region tile is 3x3
local columns. Region tiles are never stored: any rectangle is computed on
demand from the world fields and the world seed, so the same tile comes out
the same whoever asks (two embarks side by side see the same tiles at
their shared edge).

For each region tile:

- fields: every world field interpolated between world cell centres
  (smooth, Catmull-Rom), so they're continuous across cell borders;
- elevation: the interpolated elevation plus local detail: fractal noise
  sampled at the tile's absolute position, with an amplitude blended from
  the cells' biomes (2 in marsh, 4 on plains, 22 in hills and badlands, 45
  in mountains), ridged in the mountains, and stepped (terraced) where the
  rock is hard (badlands, mountains, some hills), so neighbouring tiles
  jump: cliffs;
- water: ocean where the smooth elevation plus coastal noise falls below
  sea level (so the coastline wanders across the cell, not along its
  border), lake where a lake cell's ground lies below its surface;
- biome: the biome table applied to the interpolated fields (water tiles
  are Ocean or Lake), so biome borders cut across world cells naturally;
- rivers and brooks: for every world river link, a course on the region
  tiles from the middle of the shared side (its place along the side hashed
  from the two cells, so both agree) to a jittered point near the cell
  centre, wandering by midpoint displacement. Its level falls from
  `(upstream + this)/2` at the entry side through `this` at the centre to
  `(this + downstream)/2` at the exit, so it never rises, and the tiles
  under it are cut down to it. Brooks the same, narrower;
- soil depth: deep in wet lowland, thin on drained and high ground, none on
  mountain heights;
- geology: the world is split into geological provinces (seeded Voronoi
  regions with noisy borders), each a stack of layers: sedimentary
  (sandstone, limestone) over intrusive igneous (granite), metamorphic
  (marble over granite), extrusive igneous (basalt over granite), or
  granite throughout. Mountains favour metamorphic and intrusive stacks,
  lowlands sedimentary, volcanoes extrusive. All stacks end in basalt over
  the magma sea, as before.

## 7. The local map from the region tiles

`Overworld::embark_params` computes the region tiles under the embark
footprint (and enough round it for the map to grow) and passes them in
`EmbarkParams::region`. `terrain_gen::shape` then:

1. Elevation surface: one mapping from world elevation to z-levels for the
   whole map (0.1 levels per unit, compressed only if the footprint's range
   wouldn't fit between the sea and the sky, and anchored so any water in
   the footprint sits at the map's sea level). Each column takes the
   elevation of the region tiles round it: interpolated smoothly between
   tiles of similar height, but where two neighbouring tiles differ by two
   levels or more the step is kept sharp at the tile border (wandering a
   tile with noise): a cliff. A little local noise (under a level) roughens
   it.
2. Water: region water tiles are the sea or the lake on the map.
3. Rivers: the region tiles' river courses become the river network
   (polylines through the tiles' centres, smoothed, levels from the region
   tiles), cut into their valleys as before (`rivers::carve_valley`), with
   fords; width from the flow. Brooks become the map's streams along their
   courses.
4. Soil depth and vegetation from the region tiles' fields: soil layers
   from the soil depth, tree density from vegetation.
5. Stone layers from the region tiles' geology.
6. Then the existing processes finish the land, as before: dunes, volcano,
   oasis, mesas (site features), weathering (`erosion`), sediment in the
   hollows, lakes in the hollows (`hydro`), run-off streams, everything
   joined up (`connect`), moisture, caves (`caves`), caverns and the magma
   sea, `embark_spot`.

Without region tiles (constructed `EmbarkParams` in tests, or saves made
before this change) the old path runs unchanged: relief from the world
cells' forms, the cell-link river network.

## 8. Sites, civilisations and history

They run on the new world unchanged in shape: `Race::home` gains the new
biomes (humans take shrubland too, nobody settles marsh but goblins in evil
land), lairs pick from the wild biomes, and history's random stream is
unchanged in order. They may read the fields: a capital prefers a river and
good rain for humans, and the default embark site still prefers open land
near kin with a river and no aquifer. Logging during history lowers the
vegetation round towns and hamlets.

## 9. Save compatibility

- `Biome` gains `Shrubland`, `Marsh` and `Lake`, appended to `Biome::ALL`, so
  the biome codes stored per column in `World::biomes` keep their meaning.
- New `Cell` fields (`brook`, `deep`) and new `Overworld` fields (`seed`,
  `fields`, `rivers`) are `serde(default)`; region populations are a
  `serde(default)` field of `Region`.
- An old save has no fields: `embark_params` then gives no region tiles and
  growing the map takes the old path, so the grown land still meets the old.

## 10. Performance budget

- Medium world (192x80): fields, erosion, rivers, biomes and regions under
  half a second in release; the whole world with 200 years of history in a
  few seconds. The large world stays under the existing 10 s test bound.
- Region tiles for an embark: a few milliseconds.
- Local map: within the existing bounds (`land_is_made_quickly`).

## 11. Tests

- Stage 1: biome borders follow the fields (each cell's biome is the
  table's verdict on its fields); no desert beside jungle or swamp; rain
  shadows (leeward cells of ranges drier than windward ones); salinity only
  near oceans.
- Stage 2: every river ends in the ocean, a lake or another river; flow
  grows downstream; tributaries exist; rivers are named; brooks exist and
  aren't drawn on the map; rivers never climb.
- Stage 3: regions are connected and of one broad type; the Silvery Forest
  case (one forest region of two biomes); wildlife by region; the world
  info and legends show region names.
- Stage 4: the local elevation and river and brook courses match the
  region tiles; cliffs where region tiles jump; neighbouring embarks line up
  at their shared edge; the old terrain tests adapted; `embark_spot` still
  dry and reachable; the overseer still builds (`--autoplay --runs 5
  --years 1 --seed 5000 --threads 3` before and after).
