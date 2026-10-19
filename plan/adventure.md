# Implementation plan: Adventurer mode (G03)

Grounded in the code as of v0.6.0 (`23f4963`, October 2026). File:function citations refer to that
tree. Nothing here edits the repo; every step is one commit with its test, and WISHLIST ticks go in
the same commit (CLAUDE.md).

## G03 Adventurer mode: one dwarf against the world

Goal (WISHLIST): *take one character into the world for fame, fortune or utter ruin.* Done when a
character can leave a town, find a ruin from a fallen fort, fight what lives there, and come back
with its treasure recorded in the legends.

The user's wish, in their words: "the user can take command of a single character and head off to
find fame, fortune, or utter ruin". The measure of success is the same as the fortress's: **a run
you would tell someone about**. The dwarf who limped out of the ruin of Copper Gate with the
founder's axe and bled out two cells from home. The axe lying in the grass where she fell, and
the next fort's bard singing of her.

### What the code gives us to build on

- **A site map is already a `Game`.** `Game::from_world(world, seed, ow, (sx,sy), embark)`
  (game.rs:609, private, so callers must live under `game::`) builds a bare `Game` with no
  dwarves. The arena does this (`arena/fight.rs:90 Battle::with_recruits`, a hand-built 60×22×3
  `World` from `arena/fields.rs`). Then `Battle::step` (fight.rs:334) runs its own loop instead of
  `Game::step`: it does tick, body upkeep every 10 ticks, `act(i)` per fighter, and `check_end`.
  None of the fort systems run (needs, jobs, migrants, sieges). **That is the pattern adventure
  mode extends.**
- **Land for any world cell, on demand and deterministic.** `ow.embark_params(seed, sx, sy, size)`
  (overworld.rs:1246) and `world::generate(seed ^ (sx<<20) ^ (sy<<40), &params)` (world.rs:547,
  `embark_prepared` game.rs:590). The region tiles are derived, not stored (`worldgen/tiles.rs:151`),
  so the same cell always gives the same land. One cell is 48×48 map columns. `land_is_made_quickly`
  (terrain.rs:1402) holds a Medium map (96×64×32) under 1 s in release builds. The Small map is
  64×48×32 (world.rs:11).
- **The world.** `Overworld` (overworld.rs:467) has these parts:
  - Cells: biome, evil, site, region, elev, river, volcano, brook, deep.
  - Sites: `SiteKind` is Fortress, Town, Hamlet, Retreat, DarkFortress or Lair (`:254`). A site has `ruined` and `civ` fields, and a ruined one draws as `×`.
  - Civs.
  - Figures: `Figure { name, kind, civ, born, died, cause, title, kills, lair }` (`:432`). Megabeasts have a `lair` site.
  - Events: `Event { year, text }`, plain text with no ids.
  - Regions, each with its fauna.
  - Gods, the made peoples, arts.
  - `sagas: Vec<WorldSaga>` (`:509`).

  There are several helpers: `beasts_near(x,y,r)` (`:1586`), `kill_figure` (`:1601`, already called from fort combat),
  `neighbors`, `site_at`, `region_at`, `river_at`, `person_name(rng, race)` (`:1173`). **There is
  no world tick**: `ow.year` is only written by worldgen, so nothing moves after generation.
- **Fallen forts.** A fallen fort is **not** a `Site`. What the world keeps of one:
  - its founding legend line (game.rs:784);
  - its `WorldSaga`s (`sagas.rs: saga_end` pushes them, and `sagas_fall` :707 marks the one that ended it);
  - the figures it killed.

  Its **save file stays** in `saves/` (README: "returning to the title keeps the ruin saved"). That save is a whole serialized `Game` (`saves.rs:40 write`), with the map, the items, the corpses, the artifacts, the lairs and the creatures that were still there. Its `SaveMeta` has `world` (the world's name) and `fortress`, and `Game` has `site`, `fallen` and `fortress_name`.
- **A new fort in the same world.** On the game-over page, `n` saves the fort (`AfterSave::World`, jobs.rs:16). It then opens `Mode::WorldMap` at `game.site`, and `start_embark` (app.rs:707) **clones the fallen `Game`'s overworld** into the new fort. So the world travels between games by being carried in memory, and each save holds its own copy. Adventure mode uses the same trick in both directions.
- **Combat is generic underneath.**
  - `Game::strike(&Fighter, &Fighter, &mut Body) -> Blow` (combat.rs:184) and `aftermath` (`:314`) work on any body.
  - `dwarf_fighter`/`creature_fighter`/`apply_gear` (`:122-161`) build the `Fighter` from gear, `Mat::weapon_mult`/`armor_value` and skill.
  - The entry points are type-specific:
    - `dwarf_attack(&mut Dwarf, cid)` (`:392`, which grants `fight_xp`);
    - `creature_attack(&Creature, di)`, where `di` is a **dwarves index** (`:1769`);
    - `creature_maul` (melee.rs:189), for creature against creature;
    - the arena's own `Battle::shoot` with `bolt_damage`.

  `Body` (body.rs:77) has seven parts, bleeding, `unconscious`, `limp`, `can_hold`, `press`/`bandage`/`suture`, `tick_with` every 10 ticks and healing every 200. Death goes through `Game::kill` (game.rs:1706) for a dwarf, which calls `note_death`, `hero_fell` and `saga_death`. A creature's death goes through `creature_died` (combat.rs:1319), which calls `kill_figure` and writes a legend for figures.
- **Creatures already hunt dwarves.** `update_creatures` → `creature_tick` (combat.rs:1430) picks targets by `faction`/`hostile_to` (`:67`, `:85`). Townsfolk can be `Neutral` (Visitor, Merchant), the dead are `Dead`, beasts and figures are `Monster`, and goblins and raiders are `Army`. **A hero who is a `Dwarf` in `g.dwarves` is hunted, struck and wounded by the existing code with no changes.**
- **Legendary foes.** `spawn_megabeast(figure)` (combat.rs:799) gives a named beast linked to its world figure, and a dragon breathes fire. `make_cave_horror` (`:1009`) makes a horror with powers (horrors.rs). There is also the necromancer (necromancy.rs), the thing in the night (haunt.rs), local dens (`lairs.rs: Lair`, `make_lairs`), the sealed vault with a warden (ruins.rs), and 278 bestiary animals by region (`Game::fauna`, wildlife.rs:11).
- **Deeds become legends with no new code.**
  - `credit_kill(&mut Dwarf, &Creature)` (heroes.rs:15) awards an epithet: Dragonslayer, `{Kind}bane`, Horrorbane, Warbreaker, Grim, Red-Handed.
  - `earn_epithet` (`:39`) writes the epithet, a Hero saga chapter, a legend and a message.
  - `legend(text)` (trade.rs:28) logs to `overworld.events` dated `start_year + tick/YEAR`.
  - `sagas.rs` arcs (`saga_open/add/end`, `world_saga`) become illuminated cards (`ui/saga.rs: draw_card`) and `Overworld.sagas`.
  - `life_story(&Dwarf)` (lifestory.rs:59) and `eulogy` write a life page.
  - `visitors.rs: sing` (`:118`) already picks a bard's song from `overworld.events`, so **a later fort's bards will sing of the adventurer as soon as the deeds are in the events**.
- **Off-map travel model.** In expeditions.rs, `Destination { goal: Ruin|Raid|Beast(figure), site, dist, danger }` and `destinations()` (`:168`) scan the world for lairs with a living figure, ruined sites and enemy sites within 30 cells. That gives quest targets and rumour material.
- **UI and keys.**
  - `Mode` (app.rs:205) has 50-odd variants.
  - The arena is a self-contained sub-app: `App.arena: Option<Box<ArenaScreen>>` is routed early in `on_key_inner` (app.rs:1231) before the cursor and time keys. It has its own `arena_frame` in `App::frame`, and its own `ui/arena.rs: draw_field` with a camera centred on a fighter.
  - The world map is drawn by `ui.rs: draw_world`/`paint_world`/`paint_minimap` (`:2030-2486`), the legends by `draw_legends` (`:2926`), and the saga cards by `ui/saga.rs`.
  - Smaller reusable pieces: the health colours in `ui/health.rs`, the portrait in `ui/life.rs`, `popup`, `key_line` and `look_lines`.
  - `--window` reuses `ui::draw` and `App::on_key` through a ratatui backend (window/mod.rs:111). Numpad digits arrive as `Char('1'..'9')` in both modes (window/input.rs:25). **A mode that draws through `ui::draw` works in the window for free.**
  - The title menu is `TITLE_MENU` (app.rs:20), and its indices are hard-coded in five places (app.rs:812, :896, app/arena.rs:202,206, ui.rs:4413).
- **Time.** `DAY=1200` ticks, `YEAR=12*28 days` (game.rs:157). A dwarf's step costs `MOVE_DELAY=3` ticks and a blow costs about 9. **There is no day or night**; PLAN.md's foundations table leaves the clock to "first of M6 B13 / M7 FC1".
- **What assumes a fort.**
  - `season_turns` sends migrants, sieges, caravans and mandates.
  - `sagas_tick` (sagas.rs:695) calls `sagas_fall` once every dwarf is dead.
  - App.rs:539 switches to `GameOver` on the same test.
  - `update_dwarf` runs jobs and needs.
  - Texts say `fortress_name`.

  None of these run if we don't call `Game::step`.

### Architecture

#### The choice: who runs the world?

| Option | How | For | Against |
|---|---|---|---|
| **A. `Game` with no fort, stepped by `Game::step`** | Embark a `Game` with one dwarf; gate fort systems with an `adventure` flag | Everything "just runs": water, fire, weather, wildlife, ghosts | `Game::step` and `schedule.rs` call ~50 fort systems; each needs a guard (migrants, sieges, caravans, mandates, moods, `sagas_fall`, the game-over test, `update_dwarf`'s job AI driving the hero). Guards scatter through files M2-M8 own; every new fort system is a new adventure bug |
| **B. A new world model** | Own map, own creatures, own combat | Clean | Throws away the body model, gore, bestiary, horrors, legends: the reason the mode is worth making |
| **C. Arena pattern: each site is a `Game`, driven by an adventure loop** (recommended) | `Adventure` owns the overworld, the hero and the clock; entering a site builds (or loads) a `Game` for that map with `from_world`, puts the hero in `dwarves`, and steps it with `adv_step`: a short, explicit list of the systems an adventure wants | Reuses the map, items, bodies, combat, creature AI, gore, fire, horrors, legends, sagas, life stories as they are; fort systems never run, so no guards; a **fallen fort's save is already a site `Game`**; precedent in the arena | Systems we want must be listed in `adv_step` (one line each); hero's items move between `Game`s (id remap); some `pub(super)` APIs need calling from `game::adv` (fine: it is a child module) |

**Recommended: C.** The adventure is a small driver around many short-lived `Game`s, one per
site visit, the way the arena drives a `Battle`. It reuses `Game` as a map-and-actors container
and borrows only its systems, not its loop.

```text
App ── Mode::Adventure ──► AdvScreen ─┬─ Adventure (saved as <Hero>.adv.json)
                                      │    ow: Overworld          (moved into the visit while inside)
                                      │    hero: Hero { dwarf: Dwarf, pack: Vec<Item>, coins, ... }
                                      │    clock: u64             (world time, ticks)
                                      │    at: (i32,i32)          (world cell)
                                      │    visit: Option<Visit>   (a site Game while inside one)
                                      │    memos: site memories, fame, quests, company, nemesis
                                      └─ view: Travel | Local | Talk{..} | Inventory | Status | Dead
```

- **The hero is a `Dwarf`.** `Hero` wraps a `dwarf::Dwarf` with skills, `fight_xp`, body, held weapon, armour, epithet, kills, personality and relations. Inside a site the dwarf is `visit.g.dwarves[0]`, so creatures target it through the existing code. `credit_kill`, `earn_epithet`, `kill`, `note_death`, `hero_fell`, `saga_death`, `life_story` and `eulogy` all work on it unchanged. Between sites it lives in `Adventure.hero`, the way `Expedition.members` holds dwarves off the map (expeditions.rs:36).
- **The overworld is moved, not cloned.**
  - On entering a site, `visit.g.overworld = mem::take(&mut adv.ow)`, and leaving moves it back.
  - An `Overworld` with its fields can be megabytes, so it is never copied per visit.
  - Legends written during the visit land in the one overworld.
  - `visit.g.start_year = adv.start_year` and `visit.g.tick = adv.clock` on entry, so dates run on across visits. On leaving, `adv.clock = visit.g.tick` and `ow.year = start_year + clock / YEAR`. A fort founded afterwards therefore starts after the adventure.
- **Items cross between `Game`s** through `carry_in(&mut Game, &mut Hero) -> u32 (dwarf index)` and `carry_out(&mut Game, idx) -> Hero`.
  - They take the hero's `pack: Vec<Item>` and the held and worn items, and give them fresh ids in the visit's `items` with `carried_by` set.
  - On the way out they strip the ids. Item ids are per-`Game` (`next_id`), so they are never kept across visits.
  - The same pair moves companions.

#### Time: turns made of ticks

DF's adventure mode is turn-based. We are turn-based at the player's end, with the simulation's ticks underneath:

- Every action has a cost in ticks:

  | Action | Cost |
  |---|---|
  | Step | `MOVE_DELAY`, longer with a limp (`Body::limp`), double while sneaking |
  | Blow | the dwarf's attack delay (9 + limp) |
  | Wait | 10 |
  | Get, drop or wear | 20 |
  | Talk | 50 |
  | Bind a wound | 300 |
  | Rest | until healed, interrupted, or morning |

- `adv_step` runs that many ticks, and the game then waits for the next key. Creatures move between keys at their own speeds, so a wolf (move delay 2) gains on a limping dwarf and a bear's blow lands while you fumble with your pack.
- **Interrupts.** A multi-tick action (resting, auto-walking, long-distance moves) stops when a hostile comes into sight, the hero is struck, or something new is announced.
- **`adv_step(&mut Game)`**, in game/adv/turn.rs, runs one tick of the systems an adventure wants. The list is explicit, one line per system, the same pattern as `schedule.rs`:
  - `tick += 1`;
  - `fluids` (every 3 ticks) and `fires` (`tick % 10 == 5`);
  - `smoke_tick`, `gore_tick`;
  - `horrors_tick`, `magic_tick`;
  - `ghosts_tick`;
  - `update_creatures()`;
  - `traps_tick` (a fallen fort's traps still work);
  - the hero's and companions' bodies (`tick_with` every 10 ticks, healing every 200, through `body_phase`);
  - `company_tick` (companions);
  - `folk_tick` is called from `creature_tick`.

  Not run: anything in `season_turns`, `update_dwarf`, `oversee`, `sagas_tick`'s fall test, `annals_tick`'s siege logic, and the daily fort chores.
- Each action is a few hundred ticks at most, at the cost of a fort tick with few actors (well under 0.1 ms), so a keypress stays instant.

#### The world map: travel

- Press `T` in a local map, at the map's edge or outdoors, to leave for the **travel view**: the world map (`paint_world`) with the hero's `@` in their cell, the party, the time of day and the weather.
- Moving one cell with the 8 directions (arrows or numpad) costs the walking time to cross 48 tiles of that land, set by knobs:

  | Land | Time to cross a cell |
  |---|---|
  | Grass or savanna | ½ day |
  | Forest or hills | ¾ day |
  | Mountain, swamp or jungle | 1 day |
  | Glacier | 1½ days |
  | A river with no ford | +½ day |
  | The sea | impassable |

  At night it costs ×1.5, a limp ×1.5, and blizzards or storms ×2.
- **Day and night**: `game/clock.rs` (new, pure) defines `hour(tick)`, `is_night(tick)` (20:00 to 06:00), `light(tick, underground)` and `moon(tick)`. This is the clock PLAN.md says M6 B13 or M7 FC1 will need; whichever comes first builds it, and those milestones reuse it. The local view darkens outside the hero's sight radius under the sky at night: 7 tiles instead of 20, with a torch 12.
- **Weather** while travelling is rolled per cell from its biome: wet biomes rain, cold ones snow, evil ones may have blood rain. It reuses the `Weather` enum and is drawn from the adventure's own RNG. On entering a site the weather carries into `visit.g.weather`, so the weather drawing (`ui/ambient.rs`) shows it.
- **Encounters.** Each cell step rolls for an encounter. The chance depends on:
  - savagery and evil (the cell's fields);
  - nearby lairs (`beasts_near(x,y,4)`): their figure may come for you;
  - enemy sites within 3 cells: goblin patrols, bandits (`Species::Raider`);
  - night (nocturnal beasts from the bestiary's traits);
  - the nemesis, if one is on the hero's trail;
  - fame: a famous hero draws challengers;
  - infamy: the wronged come after you.

  An encounter is announced ("Wolves! Six of them, coming out of the pines") and drops the party into a **wild site** of that cell, with the foes at the edge. Some encounters can be avoided: `s` sneaks past at night, and a hero who sees them first may press Esc to keep travelling. Every avoidance has a chance of failing.
- **Camping**: `Z` in the travel view makes camp until morning or until healed. The hero rests (healing every 200 ticks as usual) with a chance of an ambush in the night. The chance is lower near a friendly site and lower with a companion keeping watch.
- **Entering a site**: `>` or Enter on a cell. On a site cell this enters the site; elsewhere it enters the wild land of that cell. Walking off a local map's edge returns to the travel view in the neighbouring cell (DF-like).

#### Sites, built when entered

`adv/sites.rs: enter(adv, cell) -> Visit`:

- **The land** is `world::generate` with `embark_params(seed, x, y, MapSize::Small)`, the same land a fort embarked there would get. Small is 64×48×32 and takes well under a second in release, which is the budget. The loading screen (`jobs.rs: Job`) covers it, and a `JobDone::Visited(Box<Visit>)` variant is added in wave 0.
- **What stands on it** depends on what the cell holds. Each kind is a generator in its own file under `game/adv/sitegen/`:

  | Cell holds | Generator | What you find |
  |---|---|---|
  | Nothing | `wild.rs` | The land, its region's wildlife (`spawn_beasts`, one den via `make_lairs`), maybe a camp of travellers or bandits |
  | `SiteKind::Lair` with a living figure | `lair.rs` | A cave or den dug into the highest ground; the **figure itself** (`spawn_megabeast(figure)`), its hoard, and the **bones of those it killed**, named from the legends (figures whose `cause` names it; events that mention it) |
  | A ruined site (`site.ruined`) | `ruin.rs` | Broken walls and rubble in the old site's footprint (a hamlet's huts, a town's streets, a fortress's gate cut into the hill), loot by the old pop and wealth, and occupants: bandits, goblins, the restless dead (evil land), or wildlife denned in the shell |
  | A living Town, Hamlet, Fortress or Retreat | `town.rs` | Houses, a hall or tavern, a few named folk (peaceful `Species::Visitor`s with a FOLK role, or `Dwarf`s at dwarf sites), a guard or two; safe rest; later, talk, quests and trade |
  | A DarkFortress | `ruin.rs` with `hostile: true` | A goblin hold; every soul in it is hostile |
  | **A fallen fort** | `fallen.rs` | The fort's own save, see below |

- **Site memory.** A regenerated map is identical, because it comes from the same seed. What the player changed is kept as a `SiteMemo` per site key:
  - `slain`: figures and creature kinds killed;
  - `taken`: the labels of items looted;
  - `cleared: bool`, `opened: bool`;
  - `corpses: Vec<(Pos, String)>`;
  - `visited_tick`.

  It is applied after generation, so a looted hoard stays looted and a slain dragon stays dead. Its bones lie where it fell. The last two visited `Game`s are also kept whole in memory, so stepping back over an edge is instant. Only memos are saved.
- **Fallen forts.** This is the heart of the slice.
  - `Overworld` gains `forts: Vec<FortMark>`, written when a fort falls (`sagas_fall`). Each mark holds the name, cell, founded year, year fallen, the killer if known, the artifacts, and the save file. The fallen cell draws a `×` in crimson.
  - For worlds saved before this change, `fallen::scan(dir, &ow)` finds them in `saves::list` by `SaveMeta.world == ow.name` and `game.fallen`.
  - Entering one **loads its save** (`saves::read`, on the job thread). Its `Game` *is* the site:
    - the halls the overseer dug;
    - the stockpiles, still full;
    - the artifacts on their pedestals;
    - the dead where they fell;
    - the lever and the bridge;
    - the traps, still armed;
    - whatever killed it, if it was still on the map.
  - `ruin_prepare(&mut g, years)` makes it a ruin:
    - The living are gone, and every dwarf is marked dead.
    - Corpses rot to bones after a year.
    - An occupant moves in by the killer: a siege's goblins camp in the great hall; a megabeast takes the fort as its lair (its figure gets `lair` pointed at the fort, so the legends agree); unburied dead rise under a necromancer's curse; on evil land the ghosts walk (`ghosts_tick` already does ghosts); otherwise cave things climb up from the broken caverns.
    - The fort's books are left where they lay, readable with `r` while standing on one: the chronicle (`chronicles`), the overseer's notes, and the last saga.
  - The hero arrives at the old wagon spot (`g.embark`).
  - The fort's own save file is never written to. It is the player's memorial. The changes the adventurer makes are kept in the adventure's memo. *Open question 3.*

#### People: talk, news, rumours, quests

- **Talk (`k`)** to a person in sight within 3 tiles. A popup lists what you can ask:
  - **"What news?"**: rumours.
  - **"Where is …?"**: directions to sites, lairs and fallen forts the speaker has heard of.
  - **"Is there work?"**: quests.
  - **"Will you come with me?"**: companions.
  - **"Tell me of yourself"**: their name, role and one line of history.
  - **"Farewell."**
- **Rumours** are chosen from what the world knows near the speaker. People know about what lies within about 15 cells, and well-travelled folk (merchants, bards) about 40. Each rumour is a line of text, built from:
  - recent `ow.events` within the radius;
  - `ow.sagas` titles ("They still tell of *The Pride of Anvilbeard*");
  - fallen forts (`ow.forts`), with what killed them and what was said to be inside ("They say the axe of its founder is still in the deep hall");
  - living figures with lairs nearby (`beasts_near`);
  - the remains of dead adventurers;
  - the hero's own deeds, once their renown passes 10.
- Rumours are **marked on the travel map**: `Hero.known: BTreeSet<u32>` holds the site ids and fort marks the hero has been told about, and these are highlighted with a `!`.
- They are worded from the speaker's people and temper using phrase tables we write ourselves. The choice is a hash of `(speaker, topic)` through `stories::hash`/`pick`, never the game RNG. *DF's open-ended conversations are explicitly out of scope*: the idea catalogue dropped them as trivia.
- **Quests.** A quest is `Quest { id, kind, giver, at_site, target, reward, state }`:
  - **Slay** a figure, a lair's beast or a named horror. The source is `expeditions.rs: destinations` filtered to `Goal::Beast`. It is complete when `kill_figure` fires for that id.
  - **Recover** a named artifact from a fallen fort (`FortMark.artifacts`) or a ruin's vault. It is complete when the item, matched by label, is carried back to the giver.
  - **Clear** a ruin of its occupants. It is complete when no hostile is left in the site.
  - **Avenge** a fallen fort. This one is offered to its civ's folk only, and the target is whatever killed it.

  Rewards are coins, which are abstract, as recommended for M7/M8; a weapon from the giver's stock; or renown with the giver's civ. The quest log is on `j`.
- **Companions.** A person agrees to follow if the hero's renown with their civ beats their caution, or for coin. They are `Dwarf`s or creatures with a follow-and-fight routine, `company_tick` in adv/company.rs:
  - keep within 3 tiles;
  - attack hostiles in reach;
  - shoot if they carry a crossbow;
  - flee when badly hurt (`badly_hurt`);
  - leave after a defeat or an atrocity if their loyalty fails. This echoes OUT-17 ("companions turn back").

  They cross sites with the hero (`carry_in`/`carry_out`). Their deaths are chapters in the hero's saga.
- **Trade and food.** Buying in towns and hunger and thirst on the road come in the last wave. The first slice has no hunger: resting heals, and the danger is blood, not bread.

#### Fame, infamy, the hero's saga and death

- **`Fame`**: `renown: i32`, `infamy: i32`, `by_civ: BTreeMap<u32, i32>` and `deeds: Vec<Deed { tick, year, text, weight }>`.
  - Slaying a figure, a horror or a beast of a lair raises renown (`credit_kill` already gives the epithet), and so does recovering a relic or completing a quest. Killing peaceful folk, robbing a town or abandoning a companion to die raises infamy.
  - A civ the hero has wronged greets them with guards; a famous hero is greeted by name ("You're the one they call Urist Dragonslayer?").
- **The hero's saga** is a new `SagaKind::Wander` in sagas.rs, with these beats:

  | Beat | Chapters |
  |---|---|
  | Cause | "Set out from Copper Gate in the spring of 213" |
  | Escalation | each weighty deed (a figure slain, a relic found, a companion lost, a nemesis sworn) |
  | Turn | the fight of their life: the first wound that nearly killed them, or the nemesis met |
  | Ending | death (doom, crimson) or retirement (glory, gold) |

  These come up as illuminated cards (`ui/saga.rs`) as they happen. `world_saga` then writes the saga into `Overworld.sagas` with `fort` set to the hero's name and home ("Urist Dragonslayer of Copper Gate"). Titles are hashed (`title_for`).
- **Death ends the run.**
  - `Game::kill` already writes a legend, calls `hero_fell`, and sets up a spotlight. Adventure mode adds:
    - an entry in `ow.figures` for the hero (`kind: "dwarf adventurer"`, `title` the epithet, `kills`, `died`, `cause`), so the legends viewer lists them;
    - a `Remains { cell, site_key, pos, name, year, gear: Vec<String> }` in `Overworld.remains`, so a later adventurer can find the bones and the gear where they fell;
    - the Ending chapter of the Wander saga.
  - The death page shows the portrait, the life story (`life_story`), the saga card and the run's numbers (days, cells walked, foes slain, gold), and offers these keys:

    | Key | Does |
    |---|---|
    | `n` | Found a fort in this world: carries the overworld into `Mode::WorldMap`, as the fort's `n` does |
    | `a` | Another adventurer in this world |
    | `l` | Legends |
    | `t` | Title |
- **Retirement.** `R` in a friendly town settles the hero there. It closes the saga in gold, writes the hero as a living figure, and saves the world.
- **Later forts hear of you.** This happens three ways:
  1. A new fort embarks with the overworld that holds the deeds, so `start_year` follows the adventure.
  2. Bards already sing from `ow.events`.
  3. A fort that embarks on a cell with `Remains` finds the bones and gear placed on its map, through one call in `embark_prepared` that is added in wave 0.

  With M4 in place, a famous hero's civ may send them to the fort as a visitor. That belongs to M4's F07, and we leave a hook for it.

#### Saving

- `saves/<Hero>.adv.json`, written with the existing tmp-and-rename write and a `SaveMeta`. `SaveMeta` gains `#[serde(default)] kind: SaveKind { Fort, Adventure }` and appears in the load menu with an `@` mark. The file holds:
  - `Adventure` with its overworld (moved back out of the visit first);
  - the memos;
  - the hero;
  - the current `Visit`'s `Game` in full, if inside a site. A small site is a few hundred KB; a fallen fort is as big as its own save.
- Autosave on leaving a site and on each dawn while travelling. F5 saves.
- The adventure's overworld is the newest copy of the world. `n` and `a` from the death page carry it on, and a fallen fort's game-over page gets `a` to send an adventurer into *that* world. The loop is fort → ruin → adventurer → legend → fort. A separate world file is M8's FC6 and is not needed for this loop. *Open question 2.*

#### Keys (DF-style)

The adventure routes its keys before the fort's, the way the arena does (app.rs:1231). Fort letters therefore don't collide. Only the global keys are taken: Ctrl+C, Ctrl+Z, F3, `\`, and `M`/`O` for sound. `m_is_taken` (app/sound.rs:101) learns that the adventure doesn't use `M`. The bindings follow DF's adventure layout where it has one; DF uses `k` to talk and `l` to look in adventure mode, unlike fort mode's `k`.

**Local view**

| Key | Does |
|---|---|
| arrows, numpad 1-9 (and `7 9 1 3` diagonals), Home/End/PgUp/PgDn as diagonals | move / bump-attack |
| `.` or numpad `5` | wait a moment (10 ticks) |
| `<` `>` | up / down a stair or ramp |
| `A` | attack: choose a target, then a body part with odds shown |
| `f` | fire (crossbow): choose a target |
| `l` | look (a cursor; Enter for detail) |
| `k` | talk |
| `g` | get (items here; a corpse lists its gear) |
| `d` | drop |
| `i` | inventory |
| `w` | wear / wield |
| `r` | remove; on a book, read |
| `b` | bind wounds (press, then bandage with cloth) |
| `s` | sneak on/off |
| `z` | status: body, wounds, skills, fame |
| `Z` | rest until healed or interrupted |
| `T` | travel (outdoors or at the edge) |
| `j` | quests |
| `c` | companions: follow, wait, attack, dismiss |
| `S` | the hero's saga |
| `L` | legends |
| `?` | help |
| Esc | menu: save, save and quit, quit to title |

**Travel view**

| Key | Does |
|---|---|
| 8 directions | move one cell |
| `>` or Enter | enter the cell |
| `Z` | camp |
| `s` | travel quietly: fewer encounters, slower |
| `l` | world info for the cell |
| `j` | quests |
| `L` | legends |
| `?` | help |

### The first slice, and the layers on it

**Slice (waves 0-2): "out of the town, into the ruin, back with the axe".** You make a dwarf, start
in a town of your civ, and see the world map with its sites and the crimson `×` of fallen forts. You
walk there over days, through weather and night, perhaps fighting wolves on the way, and enter the
ruin of a fort you played, years later, with whatever lives in it now. You fight with the real body and wounds,
loot its stores and artifacts, and limp home or die. Either way the deeds are in the legends and
the hero's saga is on a card. Death offers a new fort or a new adventurer in the same world, which
hears of you.

**Layer 2 (wave 3): people.** Talk, rumours, quests and companions turn the world from a map into a
place.

**Layer 3 (wave 4): drama.** A nemesis who follows you, lairs that tell their history, later forts
that find your bones and sing your name, hunger on the road and trade in towns, and the tuning
batch.

### Drama hooks

These are what make a run memorable. Each belongs to a package below.

1. **Named enemies from the legends** (A4, A13). A lair's beast is its world figure. On entering, an
   arrival card reads its legend lines: "Ozthra the dragon, who burned Copper Gate in 87 and killed
   the hero Sakzul". The look panel gives its kills. Slaying it makes the hero its `{Kind}bane`
   through `credit_kill`.
2. **Lairs with history** (A13). Bones in the den are named: "the bones of Sakzul Deepaxe, who came
   here in 151", from `ow.figures` whose `cause` names the beast. Their gear lies with them and can be
   recovered, which is a quest in itself. A lair with a figure that has many kills is marked on the
   travel map as dreaded.
3. **A sworn enemy who follows you** (A12). Kill a goblin general, rob a dark fortress or flee a
   megabeast, and a **nemesis** swears on you. It is a named figure, or a new one from that civ (`person_name`), with
   kin, a grudge line, and a party that grows each season. It moves on the world map toward the hero's last
   known cell, a cell or two a day, and is an encounter when it catches up. Rumours mention it
   ("A goblin was asking after you; scarred, with a red shield"). Its first meeting is the saga's
   Turn, and killing it ends a Grudge arc. If it kills you, *it* gets the epithet and the legend
   ("Snaga Dwarfbane, who slew Urist the wanderer").
4. **Your own saga cards** (A10). The Wander arc is told as it happens, on the same illuminated cards
   the forts use.
5. **The legends remember your death** (A10). Your figure, your remains, your saga, and the legend
   line. A second adventurer can find your bones and take up your axe, and the look panel says
   whose it was. An item of a dead hero carried by a new one gains a story line, and that is a saga
   chapter ("took up the axe of Urist Dragonslayer").
6. **Later forts hear of you** (A14). Bards sing of you (automatic). A fort embarked on the cell
   where you fell finds your bones and gear. A fort of your civ may get a migrant "who walked with
   Urist Dragonslayer to the dragon's den" (a surviving companion). If your death left a quest
   undone, the overseer's notes pick it up: "They say the beast that killed Urist still sleeps in
   the hills".
7. **Return to your own fallen fort** (A5). Its halls as you left them, the overseer's last notes
   on a table, the tomb you dug, the thing that ended it now living in your great hall.
8. **Avenge it** (A8). The civ's folk offer the Avenge quest for the fort's killer.

### Order and why

1. **Wave 0 (foundations, 2.5 sessions, two agents)**:
   - The scaffolding puts in every shared type, enum value, `Mode` and save change, so later packages never touch `Game`, `Mode`, `SaveMeta`, `SagaKind` or `Overworld` again.
   - The turn engine spike proves the core, a hero in a site `Game` fighting under `adv_step`, before anything is built on it.
2. **Wave 1, the slice's body (12.5 sessions, six agents)**: making a character, the local view and keys, travel, site generation, fallen forts, and fighting as a person. All of them are in new files.
3. **Wave 2, the slice's soul (5 sessions)**: fame, death and memory; saving; and the adventure bot, which the Done-when test and tuning need. **The slice ships after wave 2.**
4. **Wave 3, layer 2 (6 sessions)**: talk and rumours, then quests, then companions.
5. **Wave 4, layer 3 (9 sessions)**: the nemesis, lairs with history, later forts, food and trade, tuning.

Critical path: W0 (scaffold, engine) → A4 (sites) → A5 (fallen forts) → A10 (memory) → the
Done-when test.

**Dependencies on other milestones.** WISHLIST says G03 needs M4's living world and M8's fallen
forts. **This plan needs neither for its slice.**
- What it needs of M8's P01 (a fallen fort as a place in the world) it builds small: `FortMark`, and loading the fort's save as the site. P01 is written so that `FortRecord` can wrap `FortMark` (see the contracts).
- Without M4 the world is frozen while you travel. When M4's F0 lands, `travel.rs` calls `world_daily()` per day of travel, and parties become encounters.
- It needs **M2 wave 0** (W0-B), for `glyphs.rs` (tile looks without ratatui, used by the local view) and for the `Mode`/`LogEntry` changes. Merge order is below.

### Release check

- `cargo test --release` green with every test named below. `one_year_timing` is unchanged: no fort code path changes, and the check proves it.
- **The determinism check.** The 25-fort batch (`--autoplay --runs 25 --years 3 --seed 5000`) before and after wave 0 gives byte-identical `--out` JSON. The scaffolding adds fields and hooks that a fort never calls; `FortMark` is written after the fall and draws no dice.
- **The adventure batch** (`--adventure-trial --runs 50 --seed 9000`, A18) has the bot set out from a town to the nearest fallen fort or lair, fight, and return. Targets:
  - 25-60% of first runs come home alive;
  - deaths are spread over the road, the site and bleeding on the way home, with none above 60%;
  - a dragon lair is entered and survived in under 15% of runs;
  - every death leaves a figure, remains and a told saga.
- **A thing to watch.** Seed 9005, the default warrior. Leave town, walk to the nearest `×`, and you should meet something on the road. Enter the fort and you should find its dead, its stores and its killer.

---

### Wave 0: shared foundations (2.5 sessions)

#### W0-A Scaffolding: types, modes, saves, hooks (1.5 sessions)

One commit adds the empty-but-working foundation and **all** shared enum values and fields G03
needs, each `#[serde(default)]`.

1. **The module tree** `src/game/adv/`, declared with one `pub mod adv;` line in game.rs. Its files:
   - `mod.rs`: `Adventure`, `Hero`, `Visit`, `View`, `SiteKey`, `SiteMemo`, `Fame`, `Deed`, `Quest`/`QuestKind` (empty variants), `Nemesis` (empty), `knob()`;
   - `turn.rs`: `Action`, `adv_step`, `act` (stubs; W0-B fills them);
   - `items.rs`: `carry_in`, `carry_out`;
   - `sites.rs`: `enter` and `leave`, with `wild.rs` as a flat open field so the engine can be tested;
   - `sitegen/{wild,lair,ruin,town,fallen}.rs`, the other four as stubs that return the wild map;
   - `travel.rs`, `talk.rs`, `quests.rs`, `company.rs`, `fame.rs`, `nemesis.rs`, `bot.rs`, each with one stub `impl` block.

   ```rust
   #[derive(Clone, Debug, Serialize, Deserialize)]
   pub struct Adventure {
       pub seed: u64,
       pub ow: Overworld,              // empty while a visit holds it
       pub start_year: i32,
       pub clock: u64,                 // world ticks since the adventure began
       pub at: (i32, i32),             // world cell
       pub hero: Hero,
       #[serde(default)] pub company: Vec<Hero>,
       #[serde(default)] pub visit: Option<Box<Visit>>,
       #[serde(default)] pub memos: BTreeMap<SiteKey, SiteMemo>,
       #[serde(default)] pub fame: Fame,
       #[serde(default)] pub quests: Vec<Quest>,
       #[serde(default)] pub nemesis: Option<Nemesis>,
       #[serde(default)] pub weather: Weather,
       #[serde(default)] pub log: Vec<LogEntry>,     // travel log; a visit uses its Game's
       #[serde(default)] pub knobs: Knobs,
       #[serde(default)] pub stats: AdvStats,        // days, cells, slain, gold: for the end page and the batch
       #[serde(default)] pub over: Option<AdvEnd>,   // Died { cause, tick } | Retired { tick }
       rng: Rng,                                     // the adventure's own: travel, encounters, weather
   }
   #[derive(Clone, Debug, Serialize, Deserialize)]
   pub struct Hero {
       pub dwarf: Dwarf,                // body, skills, fight_xp, epithet, kills; id 0 outside a visit
       #[serde(default)] pub pack: Vec<Item>,  // carried, held and worn items; ids meaningless outside a visit
       #[serde(default)] pub coins: u32,
       #[serde(default)] pub home: Option<u32>,          // site id
       #[serde(default)] pub calling: Calling,           // Warrior, Hunter, Wanderer, Peasant (A1)
       #[serde(default)] pub known: BTreeSet<u32>,       // sites and fort marks heard of (A7)
       #[serde(default)] pub loyalty: i32,               // for companions
   }
   #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
   pub enum SiteKey { Wild(i32, i32), Site(u32), Fort(u32) /* index into ow.forts */ }
   pub struct Visit { pub key: SiteKey, pub g: Game, pub hero: usize /* dwarves index */ }
   pub enum View { Local, Travel, Look(Pos), Talk { who: u32, sel: usize }, Inventory { sel: usize },
                   Status, Quests, Company, Saga, Dead, Menu { sel: usize } }   // App-side, not saved
   impl Adventure {
       pub fn new(ow: Overworld, seed: u64, hero: Hero, home: u32) -> Adventure;
       pub fn ow(&self) -> &Overworld;          // the visit's while inside one
       pub fn knob(&self, name: &'static str, default: f64) -> f64;
       pub fn hour(&self) -> u32;               // via clock.rs
   }
   ```
2. **`src/game/clock.rs`** (new, pure): `hour(tick) -> u32`, `is_night(tick) -> bool`, `light(tick, under_sky: bool) -> u8` (0-255), `moon(tick) -> u8` (phase 0-7) and `time_words(tick) -> &'static str` ("before dawn", "midday"…). PLAN.md's clock foundation; M6 B13 and M7 FC1 extend it rather than build another.
3. **Overworld** (overworld.rs): `#[serde(default)] pub forts: Vec<FortMark>` and `#[serde(default)] pub remains: Vec<Remains>`.

   ```rust
   pub struct FortMark { pub name: String, pub x: i32, pub y: i32, pub civ: Option<u32>,
       pub founded: i32, pub fell: Option<i32>, pub killer: Option<String>,
       pub artifacts: Vec<String>, pub save_file: String, pub peak_pop: u32 }
   pub struct Remains { pub name: String, pub x: i32, pub y: i32, pub site: Option<u32> /* site id, None in the wild or a fort */,
       pub pos: (i32, i32, i32), pub year: i32, pub gear: Vec<(ItemKind, Mat, String)>, pub figure: u32 }
   impl Overworld {
       pub fn fort_at(&self, x: i32, y: i32) -> Option<usize>;
       pub fn add_figure(&mut self, f: Figure) -> u32;   // public; history's own pushes stay private
   }
   ```

   `sagas.rs: sagas_fall` pushes the `FortMark`: it already runs once, when the fort falls, and it draws no dice. The world map draws a fallen fort's cell as a crimson `×` (`paint_world`, one arm).
4. **`SagaKind::Wander`** in sagas.rs, with its `name()` and `told()` arms (told: two chapters and weight ≥ 2, or ended in death after an Escalation). `pub(super) fn saga_wander(&mut self, beat, text, weight)` is keyed on the hero's dwarf id.
5. **Saves** (saves.rs):
   - `SaveMeta.kind: SaveKind` (default `Fort`);
   - `write_adventure(dir, &Adventure, autosave, report) -> Result<SaveMeta, String>` and `read_adventure(dir, file, report) -> Result<Adventure, String>`;
   - `list` returns both kinds.

   The load menu shows an `@` for adventures, and `start_load` dispatches on `kind`.
6. **App**:
   - `Mode::Adventure`, with `App.adv: Option<Box<AdvScreen>>` (in the new `src/app/adventure.rs`, after the arena's pattern: `struct AdvScreen { adv: Adventure, view: View, cursor: Pos, … }`);
   - one early-route arm, `Mode::Adventure => return self.key_adventure(k)`;
   - one `frame` arm, `adv_frame()`;
   - `JobDone::Visited(Box<Adventure>)` for site loads and `JobDone::AdvLoaded`.

   The title menu gains **"Adventure"** after "Watch a fort". The hard-coded menu indices (app.rs:812, :896, app/arena.rs:202,206, ui.rs:4413) become named consts (`TITLE_NEW`, `TITLE_CONTINUE`, …). The game-over page gets `a`, "send an adventurer into this world": it saves first, as `n` does, then carries the overworld.
7. **UI**: one `Mode::Adventure => adventure::draw(f, app)` arm in `ui::draw`, in the new `src/ui/adventure.rs` (W0 draws "the adventure begins…" and the hero's `@` on a plain field), and the title's detail line.
8. **The embark hook for remains**: one line in `embark_prepared` after `make_lairs`, `self.place_remains()`, defined in `game/adv/fame.rs` (W0 leaves it empty, and A14 fills it).
9. **`main.rs`**: `--adventure-trial` is parsed and passed to `game::adv::bot::trial_batch` (a stub, filled by A18).
10. **Window**: `--window --screenshot out.png --adventure [--seed N]` opens a seeded adventure in a town and saves the frame (window/mod.rs `screenshot`, one flag).

- **Test:**
  - `adv/mod.rs: an_adventure_survives_a_save` (build an `Adventure` from `Game::new(5).overworld`, then write, read and compare);
  - `saves.rs: old_saves_load_without_adventure_fields` (a fixture without `forts`, `remains`, `kind`);
  - `clock.rs: night_falls_and_the_moon_turns`;
  - `sagas.rs: a_fallen_fort_leaves_its_mark` (`Game::new(5)`, kill everyone, step: `ow.forts` has the fort with its cell and `save_file`);
  - `app.rs: the_title_offers_an_adventure` (an App test: down to Adventure, Enter, `Mode::Adventure`);
  - the coordinator's byte-identical 25-fort batch.
- **Hot spots** (why it goes alone and first):
  - game.rs: one `mod` line.
  - overworld.rs: 2 fields, 2 fns, 1 draw arm.
  - sagas.rs: 1 variant, 2 arms, 1 fn, and `FortMark` in `sagas_fall`.
  - saves.rs: 1 field, 2 fns.
  - app.rs: 1 Mode, 2 arms, the title consts, the game-over `a`.
  - app/arena.rs: the title consts.
  - ui.rs: 2 arms and the title consts.
  - main.rs: 1 flag.
  - window/mod.rs: 1 flag.
  - jobs.rs: 2 variants.
  - game.rs `embark_prepared`: 1 line.

  This is done once, by one agent, **after M2's W0-B has merged** (see "Interaction with Milestone 2").

#### W0-B The turn engine: a hero in a site `Game` (1 session; parallel with W0-A, rebased on it)

This spike de-risks the whole mode: the hero acts, the world answers, and blood is drawn.

- **Design** (game/adv/turn.rs, game/adv/items.rs):

  ```rust
  pub enum Action { Move(i32, i32), Climb(i32 /* +1 up, -1 down */), Wait(u32), Attack { target: Target,
      part: Option<Part> }, Fire { target: Target }, Get(Vec<u32>), Drop(Vec<u32>), Wear(u32),
      Remove(u32), Bind, Rest, Talk(u32) }
  pub enum Target { Creature(u32), Dwarf(u32) }
  pub enum Outcome { Done { ticks: u32 }, Interrupted { ticks: u32, why: String }, Refused(String), Died }
  impl Game {
      /// One tick of what an adventure runs (the list is the contract: one line per system).
      pub(crate) fn adv_step(&mut self);
      /// The hero (dwarves[i]) does `a`; the world runs for its cost; stops early on an interrupt.
      pub(crate) fn act(&mut self, hero: usize, a: Action) -> Outcome;
      /// What would make a long action stop: a hostile newly in sight, a blow taken.
      pub(crate) fn alarm(&self, hero: usize, since_tick: u64) -> Option<String>;
      /// Creatures the hero can see (line of sight on its z, radius from clock::light).
      pub(crate) fn in_sight(&self, hero: usize) -> Vec<u32>;
  }
  pub fn carry_in(g: &mut Game, h: Hero, at: Pos) -> usize;   // fresh ids, carried_by, held/armor re-pointed
  pub fn carry_out(g: &mut Game, i: usize) -> Hero;           // items stripped into pack
  ```

  - `Move` into a hostile is a bump attack, which calls the existing `dwarf_attack(&mut d, cid)` (`fight_xp` +8, wounds, `credit_kill` on a kill). `Attack` with a `part` is passed to A6's aimed blow; W0-B uses `dwarf_attack`.
  - The cost of a move is `MOVE_DELAY + limp`. A blow costs `9 + limp`, the existing dwarf attack delay.
  - The hero gets `Task::Idle` and never runs `update_dwarf`, so it moves only through `act`.
  - Death comes through `Game::kill` as usual, and `act` returns `Died` when `!dwarves[hero].alive`.
  - `adv_step` must not call `sagas_tick` (its fall test would end a fort that never was). Its list:
    - `tick += 1`;
    - `fluids` (every 3 ticks) and `fires` (`tick % 10 == 5`);
    - `smoke_tick`, `gore_tick`;
    - `horrors_tick`, `magic_tick`;
    - `ghosts_tick`;
    - `update_creatures()`;
    - `traps_tick`;
    - every 10 ticks, `body.tick_with` for each living dwarf, plus `blood_trail` and `kill` on death (as `Battle::step` does);
    - every 200 ticks, healing (the existing heal in `body_phase`);
    - `company_tick()` (empty until A9).
- **Steps:**
  1. *Engine and items* (turn.rs, items.rs). Test `turn.rs: a_hero_steps_and_the_wolf_comes`: a flat site, the hero at one end and a hostile wolf at 8 tiles, `act(Wait(10))` repeated. The wolf closes in and strikes. The hero's body shows a wound within 200 ticks, and the log says so.
  2. *Bump to fight, die properly.* Test `turn.rs: a_hero_with_an_iron_axe_kills_a_goblin_more_often_than_not` (50 seeds, the hero bump-attacks every turn, ≥ 60% win; a death goes through `kill`, writes a `DeathNote` and returns `Died`) and `items.rs: gear_crosses_between_games` (carry_in to game A, carry_out, carry_in to game B: the same kinds, materials, labels and held/worn state; ids differ).
- **Hot spots:** none outside `game/adv/`. It calls `pub(super)` combat functions from a child module of `game`.

---

### A1 Make a character  (effort: 1.5 sessions; risk: low)

- **Design** (game/adv/create.rs, ui/adv_create.rs, app/adventure.rs `key_create`):
  - The title's "Adventure" first asks **which world**:
    - **"A new world"** runs the world setup and generation, then comes here instead of the world map. A `App.after_worldgen: AfterGen { Embark, Adventure }` flag is added in W0.
    - **One of the worlds in `saves/`**: worlds grouped by `SaveMeta.world`, the newest save's overworld taken. The list shows the fallen forts each holds.
  - **Race**: dwarf (the slice). *Open question 1.*
  - **Calling**, a starting budget of skill points and kit:

    | Calling | Skill | Kit |
    |---|---|---|
    | Warrior | fighting, adequate | copper axe or spear, leather armour, shield |
    | Hunter | fighting, novice | crossbow, 20 bolts, a knife (pick) |
    | Wanderer | a little of each | walking staff (wood hammer), cloak, 30 coins |
    | Peasant | none | a pick, 5 coins: ruin is likelier, glory greater |

    Points can be moved between fighting and the labour skills (←→), like *Prepare carefully* (`loadout.rs`'s budget pattern: 150 points, costs from `skill_cost`).
  - **Name** is rolled with `person_name` and can be edited (`n`, the world-setup text field's pattern). **Looks and personality** come from `new_dwarf`; `r` rerolls them. The portrait is shown (`ui/life.rs: portrait`).
  - **Home** is a living site of the hero's civ: the nearest Hamlet, Town or Fortress to a fallen fort if the world has one, otherwise a random one. `[` `]` cycle through them, and the travel map preview shows where it is.
  - The adventure starts at `ow.year + 1`, spring, at dawn, in the home's town map.
- **Steps:**
  1. *The creation screen and the world choice.* Test `create.rs: every_calling_makes_a_hero_with_its_kit` and `ui: make_a_hero_by_keys` (App test: title, Adventure, new world on seed 5 headless, Enter through to `Mode::Adventure` with the hero in a town).
  2. *Home and start.* Test `create.rs: the_hero_starts_at_home_in_spring` (site of their civ, `clock` 0, `hour == 6`).
- **Parallel work:**
  - **Writes:** create.rs, ui/adv_create.rs, app/adventure.rs (`key_create` only).
  - **Reads:** W0-A.
  - **Hot spots:** none.

### A2 The local view and its keys  (effort: 2 sessions; risk: medium)

- **Design** (ui/adventure.rs, app/adventure.rs `key_local`):
  - **The map** is centred on the hero (`ui/arena.rs: camera`/`draw_field` is the template). Tiles come from `glyphs::tile_look` (M2 W0-B), and the hero is a bright `☺` in the calling's colour. Creatures, items, blood and webs use the fort's layers, through `glyphs::unit_glyph`.
  - **Sight**: the hero's z only. Tiles in line of sight within `in_sight`'s radius are bright; remembered tiles are dim, using `Tile.revealed`. At night under the sky the radius shrinks, and caverns are lit only by the hero's torch.
  - **Sidebar** (38 columns, like the fort's):
    - name and epithet;
    - the body as `ui/health.rs` colours the parts, with bleeding shown as "bleeding badly";
    - held and worn items;
    - the time words and the weather;
    - the site's name;
    - companions;
    - the active quest.
  - **Bottom**: the visit's announcements (`g.log`, the fort's box), with combat lines coloured by kind once M2's `LogKind` lands.
  - **At 80 columns** the sidebar folds under the map into two lines.
  - **Keys**: the local table above. Movement takes arrows, numpad digits as `Char`, `7 9 1 3` and Home/End/PgUp/PgDn as diagonals. `l` opens a look cursor (the fort's `look_lines`). `i`, `w`, `r`, `g` and `d` open small popups (`popup`) that list items with letters a-z, DF-style. `z` is the status page. `?` is help. Esc opens the menu.
  - **The frame**: `adv_frame` does nothing while waiting for a key. During a long action (rest, auto-walk) it runs up to 600 ticks a frame and stops on `Interrupted`.
- **Steps:**
  1. *Draw and move.* Tests: `ui: render_adventure_snapshot` (80×24 and 130×40 text snapshots of a seeded town: the hero, the sidebar and the log all appear, and nothing overflows) and `app: keys_move_the_hero` (each of the 8 directions by arrows and by numpad digits moves one tile; `.` advances 10 ticks).
  2. *Look, inventory, wear, get, drop, status, help.* Test `app: pick_up_wear_and_drop_by_keys` (a helm on the ground: `g a`, `w a`, then status shows it worn; `d a` drops it).
  3. *Window.* `--window --screenshot --adventure` works. Test `window: adventure_screenshot_draws` (behind the `window` feature: a non-empty frame with the hero's cell lit).
- **Parallel work:**
  - **Writes:** ui/adventure.rs, app/adventure.rs (`key_local`, popups).
  - **Reads:** W0, M2's glyphs.rs.
  - **Hot spots:** none in shared files. `m_is_taken` gets one arm in W0.

### A3 Travel the world  (effort: 2.5 sessions; risk: medium)

- **Design** (game/adv/travel.rs, ui/adv_travel.rs, app/adventure.rs `key_travel`):
  - **The view**: `paint_world` (made `pub(crate)` in W0 if it isn't already) with the hero's `@` blinking. Known rumour sites are marked `!`, fallen forts with a crimson `×`, and the nemesis's last sighting with a red `?`. The side panel holds the cell's `world_info` lines, the time, the weather and the party.
  - **Steps**: `Adventure::travel(dx, dy) -> TravelOutcome { Arrived, Encounter(Encounter), Blocked(String) }`. The cost comes from the cell's biome and river by knob (`travel.grass_days` … `travel.glacier_days`, `travel.night_mult`, `travel.limp_mult`). The clock advances, and the hero's body ticks too: **a bleeding hero keeps bleeding on the road**. That is the long walk home with an open wound, the slice's best drama, and it needs no new code beyond calling `tick_with` per 10 ticks of travel (cheap: a day is 120 calls).
  - **Weather** is rolled once per half-day for the current cell: `travel.rs: roll_weather(cell, season, rng)`.
  - **Encounters**: `fn encounter_chance(&self, cell) -> f64` adds up the factors listed in the architecture, each a knob. `fn roll_encounter(&mut self) -> Option<Encounter { kind, foes: Vec<(Species, u8)>, figure: Option<u32>, text }>`. The foes come from the region's fauna (hunters and predators of the right size), goblin or bandit patrols near enemy sites, the undead on evil land at night, and a lair's figure within 4 cells (rare, terrible).
  - **Camping**: `camp()` rests until dawn, healing, with a night-ambush roll at half the encounter chance.
  - **The edges**: leaving a local map by its edge travels to the neighbour cell, and entering a cell puts the hero on the edge they came from (`sites::enter_from(dir)`).
- **Steps:**
  1. *Move, time, night and weather.* Test `travel.rs: a_day_across_the_grass_and_three_across_the_mountains`.
  2. *Encounters and camping.* Test `travel.rs: savage_land_is_dangerous` (10,000 rolls on a calm and a savage cell: the savage rate is at least three times the calm one; at night it is higher) and `travel.rs: a_wounded_hero_bleeds_on_the_road` (a gash, travel 3 cells: blood is lower, and with a deep wound the hero can die on the road, `AdvEnd::Died { cause: "bled to death on the road to …" }`).
  3. *The travel screen and keys.* Test `app: travel_by_keys` (`T`, 3 moves east, Enter: in a local map of the right cell).
- **Parallel work:**
  - **Writes:** travel.rs, ui/adv_travel.rs, app/adventure.rs (`key_travel`).
  - **Reads:** W0 (clock, Adventure), A4's `enter` (stubbed until A4 merges: the wild field).
  - **Hot spots:** none.

### A4 Sites, built when entered  (effort: 3 sessions; risk: medium-high)

- **Design** (game/adv/sites.rs, sitegen/{wild,lair,ruin,town}.rs):
  - **The entry points**: `enter(adv, key, from: Option<Dir>) -> Result<(), String>` and `leave(adv) -> SiteKey`. Entering runs on the job thread with the loading screen. `land(ow, seed, x, y) -> World` generates with `MapSize::Small`, and `from_world` then makes the `Game`. The site `Game`'s `fortress_name` is set to the site's name, or the region's ("the Pines of Ogre Hill"), so the existing texts read well ("Urist of Copper Gate", "in the Pines of …").
  - **wild.rs**: the region's wildlife (`spawn_beasts` from `Game::fauna`), one den (`make_lairs` with the game's own seed), and an encounter's foes at the edge if given.
  - **lair.rs**:
    - Find the highest rock near the centre and dig a den. It is a cave of 10-30 open tiles reached by a tunnel, with ramps down into the rock (the terrain helpers in `terrain_gen/caves.rs` where they fit; a straight dig otherwise).
    - `spawn_megabeast(figure)` deep in the den, with its hoard: gold, gems and items by its `kills`, and an artifact if its history killed a hero.
    - Its victims' bones are placed as items labelled from the legends: figures whose `cause` names it, and `ow.events` that mention it.
    - The approach shows an arrival card (`spotlight`): the figure's name, kind, kills and legend lines.
  - **ruin.rs**: the old site's footprint by kind. A hamlet is 5-8 huts of 4×4 to 6×5. A town is streets, a market and a hall. A fortress is a gate cut into the nearest hillside with a few halls behind it. A dark fortress is towers. Walls are left broken (40% gone) and the floors are rubble.
    - Loot is placed by the old `pop`.
    - Occupants by the evil field, the savagery and the years since the fall:

      | Condition | Occupant |
      |---|---|
      | Evil land | the dead (`Species::Undead`) |
      | A dark fortress | goblins |
      | Near a road or another site | bandits (`Raider`) |
      | Otherwise | a den of the region's hunters |

    - A one-in-five chance of a sealed vault (`place_vault`, the existing one), with its warden.
  - **town.rs**: a living site, with houses and a central hall or tavern.
    - Folk number 6-14 by `pop`. They are `Species::Visitor` creatures given a `FOLK` role constant in adv. `creature_tick` gains one early line: `if c.species == Visitor && c.visitor == FOLK { return self.folk_tick(c) }`, with `folk_tick` in adv/sitegen/town.rs (wander near home, flee from hostiles, gather at the hall at night).
    - Guards (2-4, the civ's race, armed) are folk with a GUARD role, who fight hostiles and anyone with infamy here.
    - At a dwarf civ's site the folk are `Dwarf`s instead: `Task::Idle` dwarves with the same `folk_tick` through a `dwarves` loop in `adv_step` (one line).
    - Names come from `person_name` and roles from a small table (smith, brewer, elder, hunter, bard).
    - A town is safe: resting there has no ambush roll.
  - **Site memory**: `SiteMemo` is applied after generation, and site generation is deterministic in `(world seed, key)`. A slain figure is not respawned: its corpse, or bones after a season, lies where it fell (`corpses`). Looted items are not re-placed, matched by label (artifacts) or by kind and position (ordinary loot). `cleared` sites spawn no occupants for a year. The last two `Game`s are kept whole (`Adventure.recent: Vec<Visit>`, `#[serde(skip)]`).
- **Steps:**
  1. *Land on demand and the edges.* Test `sites.rs: the_same_cell_gives_the_same_land` (enter, leave, enter: identical tiles) and `sites.rs: entering_is_quick` (release, Small, under 1.2 s on the CI machine, `#[ignore]`d in debug).
  2. *Lairs.* Test `lair.rs: the_dragon_waits_with_its_hoard_and_its_victims` (a world with a dragon figure that has kills: enter its lair; the creature has `figure == Some(id)`, the hoard has items, and at least one bone item names a victim if the history has one).
  3. *Ruins and towns.* Tests `ruin.rs: a_ruin_has_walls_loot_and_tenants` and `town.rs: townsfolk_are_peaceful_and_guards_fight` (a wolf spawned in town: guards attack it, folk flee; the hero striking a folk turns the guards hostile and adds infamy).
  4. *Memory.* Test `sites.rs: a_slain_beast_stays_slain` (kill the lair figure with `kill_for_test`, leave, enter: no figure creature, bones at its spot, and `ow.figures[f].died` set).
- **Parallel work:**
  - **Writes:** sites.rs, sitegen/{wild,lair,ruin,town}.rs; combat.rs `creature_tick` (one line).
  - **Reads:** W0, W0-B; terrain_gen (read only), ruins.rs `place_vault`, lairs.rs `make_lairs`, combat.rs `spawn_megabeast`.
  - **Hot spots:** combat.rs, one line. M2 doesn't own combat.rs; M4's P0 splits it, so if that has happened, the line goes into the split file that holds `creature_tick`.

### A5 Fallen forts: the ruin you made  (effort: 2 sessions; risk: medium)

- **Design** (game/adv/sitegen/fallen.rs):
  - **Finding them**: `ow.forts` (W0). For worlds from before W0, `fallen::scan(dir, &mut ow)` reads `saves::list`, takes each save with `world == ow.name`, loads it lazily (only `fallen`, `site`, `fortress_name`, the artifacts and the year), and appends `FortMark`s. It runs once, when an adventure starts.
  - **Entering one**: `saves::read(dir, mark.save_file)` on the job thread. If the file is missing, the fort is generated as an ordinary ruin from its mark (ruin.rs, fortress footprint), with a note: "Nothing is left but the gate".
  - **`ruin_prepare(g: &mut Game, years: i32, mark: &FortMark, ow: &Overworld)`**:
    - Every dwarf is marked not alive, so their corpses stay as they are.
    - After a year, corpses and carcasses become bones (`rot_daily`'s logic run forward).
    - `g.overseer = None`, and the squads, alerts, caravans, captives and expeditions are cleared.
    - Fires are put out.
    - The creatures still on the map stay: the siege that killed it may still be camped there.
    - An **occupant** is added by the killer and the years (it matches M8 P01's `Occupant`):
      - an army: a band of that civ in the main hall;
      - a beast or figure: it moves in, and its `Figure.lair` is pointed at the fort;
      - a horror: it nests in the deepest level;
      - the dead: the unburied rise (necromancy's raise, if the world has a necromancer near) or ghosts walk (`ghosts_tick`);
      - nothing: cave things come up from opened caverns, and wildlife dens in the outer halls.
    - **The books**: the year's chronicle pages (`g.chronicles`), the overseer's last notes and the saga that ended it are placed as readable items (`ItemKind::Book` is not added: the slice uses a labelled `Crafts`/stone tablet item whose label carries the text key) on the nearest table or the floor of the main hall. `r` on it shows the text in a popup.
  - **Arrival**: the hero comes in at `g.embark`, the old wagon spot. A card says: "Copper Gate. It fell in 213, to the goblins of Snagrod. Seven years of silence."
- **Steps:**
  1. *Scan and load.* Test `fallen.rs: a_fallen_fort_is_found_and_entered` (run `Game::new(5)` 10 days, `kill_for_test` everyone, step until fallen, save to a temp dir; scan finds it; enter loads it; the map tiles equal the save's; at least one artifact or the stockpiles are present).
  2. *Ruin and occupant.* Test `fallen.rs: what_killed_it_lives_there_now` (a fort fallen to a megabeast: the beast is in the fort and its figure's lair is the fort mark; a fort fallen to goblins: goblins are present).
  3. *Books.* Test `fallen.rs: the_overseers_last_notes_are_left_behind`.
- **Parallel work:**
  - **Writes:** sitegen/fallen.rs.
  - **Reads:** saves.rs, W0 `FortMark`, A4's `enter` (dispatch by `SiteKey::Fort`).
  - **Hot spots:** none.

### A6 Fighting as a person  (effort: 1.5 sessions; risk: medium)

- **Design** (game/adv/fight.rs):
  - **Aimed blows** (`A`): choose a target in reach, then a part. The odds come from `strike`'s hit chance, with a penalty for a small part (head −20%, arms −10%). The aimed part is passed to `strike` through a new `pub(super) fn strike_at(att, def, body, part: Option<Part>)` beside `strike` in combat.rs. When `part` is `None` it calls the old code, so forts are unchanged.
  - **Shooting** (`f`): the arena's `Battle::shoot` logic, lifted into a `pub(super) fn bolt_at(&mut self, from: Pos, at: Target, skill, mat) -> Option<Blow>` in fight.rs, reusing `bolt_damage` and `clear_shot`.
  - **Binding** (`b`): `Body::press` while acting, then `bandage` if the pack holds cloth or thread. A field dressing as in `expeditions.rs: field_dressing` otherwise. It costs 300 ticks and is interrupted by an attack.
  - **Down and out**: an unconscious hero (`Body::unconscious`) cannot act. The world runs on in 50-tick chunks until the hero wakes or dies. Companions may drag them (A9). A downed hero is hit almost surely (the `opening.hit` knob).
  - **Fleeing works**: hostile beasts lose interest at `aggro*2+2` tiles (creature_tick), so running is a real choice.
  - **Looting**: `g` on a corpse tile lists the corpse's gear (items whose `carried_by` was the dead creature, dropped by `die`/`creature_died`) and everything else lying there.
- **Steps:**
  1. *Aim and shoot.* Test `fight.rs: aiming_at_the_head_hits_less_and_kills_more` (2,000 headless blows against a goblin body).
  2. *Bind, fall, loot.* Test `fight.rs: a_bound_wound_stops_bleeding` and `fight.rs: a_slain_goblins_gear_can_be_taken`.
  3. *Done-when for combat.* Test `fight.rs: bare_hands_lose_to_a_bear_iron_wins_against_a_goblin` (50 seeds each: bare-handed against a grizzly loses ≥ 90%; an iron axe with mail against one goblin wins ≥ 70%).
- **Parallel work:**
  - **Writes:** adv/fight.rs; combat.rs (`strike_at`, a new fn beside `strike`).
  - **Reads:** W0-B.
  - **Hot spots:** combat.rs gets a new function only. A4's one line is in `creature_tick`, a different function.

---

### A10 Fame, the hero's saga, death and memory  (effort: 2.5 sessions; risk: medium) — wave 2

- **Design** (game/adv/fame.rs, ui/adv_end.rs):
  - **Deeds**: `fn deed(&mut self, text, weight)` adds a `Deed`, raises renown by its weight, and writes the Wander saga chapter (`saga_wander`, W0) and, at weight ≥ 3, a legend.
    - It is called from: a figure slain (hooked by comparing `ow.figures[..].died` before and after each `act`, so nothing in combat.rs changes); an artifact picked up (`quality == ARTIFACT`); a fallen fort entered and left alive; a lair cleared; an epithet earned (the hero's `epithet` changing); a quest done (A8); a companion lost (A9); the nemesis sworn or slain (A12).
  - **Renown and infamy by civ** are stored for A7, A8 and A9 to read. A first version of the effects: a guard greets the famous, and the infamous are attacked on sight in that civ's sites.
  - **Death**: `act` → `Died`, which calls `fn hero_died(&mut self, cause)`:
    1. a figure for the hero (`ow.add_figure`);
    2. `Remains` with their gear (the items on their tile);
    3. the Wander saga ends in doom, with the closing line from `eulogy`;
    4. a legend line ("Urist Dragonslayer of Copper Gate fell in the ruin of Anvilbeard, slain by a goblin, in 214.");
    5. `AdvEnd::Died`, and an autosave of the dead adventure.
  - **The death page** (ui/adv_end.rs) shows the portrait, the life story (`life_story` on the hero's `Dwarf`), the saga card (`ui/saga.rs: draw_page`), and the run's numbers (`AdvStats`). Its keys are `n`, `a`, `l` and `t`, as in the architecture.
  - **Retirement**: `R` in a friendly town settles the hero. The saga ends in gold, and the hero becomes a living figure.
  - **`S`** shows the hero's saga so far, through the same `draw_page`.
- **Steps:**
  1. *Deeds and renown.* Test `fame.rs: slaying_a_figure_is_a_deed_and_a_legend`.
  2. *Death and the world's memory.* Test `fame.rs: the_world_remembers_the_dead` (a hero killed in a lair: `ow.figures` holds them, dead, with an epithet if they had one; `ow.remains` has their gear; `ow.sagas` has a Wander saga with `doom`; `ow.events` has the line).
  3. *The end page, and on into the world.* Test `app: from_death_to_a_new_fort` (death, `n`, the world map, embark: the new fort's `overworld.sagas` holds the hero's saga and `start_year` is after the death).
  4. **G03 Done-when** test `adv/tests.rs: a_hero_brings_home_the_treasure_of_a_fallen_fort`: seed a world, run a fort to its fall (the A5 fixture), start an adventure at the nearest town, and let the bot (A18) travel to the fort, fight what lives there, take an artifact or the best item, and travel home. Up to 20 seeds may be tried. Then assert at least one seed brings the treasure home and every seed leaves a legend line, a deed and (for the dead) remains.
- **Parallel work:**
  - **Writes:** fame.rs, ui/adv_end.rs, app/adventure.rs (`key_end`).
  - **Reads:** W0, A2-A5.
  - **Hot spots:** none.

### A11 Saving and loading  (effort: 1 session; risk: low) — wave 2

- **Design**:
  - `write_adventure` takes the overworld back out of a visit before serializing and puts it back after; a `Visit` is saved whole.
  - Autosave on leaving a site, at each dawn on the road, and on death. F5 saves and F9 reloads.
  - The title's Continue picks the newest save of either kind, and the load menu lists both.
  - An adventurer save on a fallen fort holds that fort's `Game`, so it can be large; the meta shows the size.
- **Test:** `saves.rs: an_adventure_inside_a_fallen_fort_saves_and_loads` (enter the A5 fixture, save, load: the same tick, hero, site and overworld events).
- **Writes:** saves.rs (the two functions from W0, filled), app/adventure.rs (F5/F9 and Esc menu).

### A18 The adventure bot and its batch  (effort: 1.5 sessions; risk: medium) — wave 2

The Done-when test and every tuning goal need adventures that play themselves (PLAN.md: "balance work is measured, not written").

- **Design** (game/adv/bot.rs):
  - `fn bot_turn(adv) -> Action`, with simple aims:
    - pick the nearest fallen fort, or failing that a lair, ruin or quest target;
    - travel there, camping when hurt;
    - inside, walk toward the nearest hostile or loot, bump-attack, bind wounds below half blood, and flee to the edge below a third;
    - take the best item, then travel home.
  - `trial(seed, opts) -> AdvTrial { home, target, came_home, died_where: Road|Site|Home, cause, killer, days, slain, loot, deeds }` (serde).
  - `trial_batch` with `--runs`, `--seed`, `--threads` and `--out`, plus a report section in `report.rs: adventures(trials)`. That is one new function; `report.rs` is touched by M2's W0-B, so this merges after it.
- **Test:** `bot.rs: the_bot_goes_and_comes_back` (on a fixed seed, a calm world and a weak target: it returns home).
- **Writes:** bot.rs, report.rs (one fn), main.rs (the W0 flag wired up).

---

### A7 Talk, news and rumours  (effort: 2 sessions; risk: medium) — wave 3

- **Design** (game/adv/talk.rs, ui/adv_talk.rs):
  - **The conversation popup**, as in the architecture.
  - **`fn rumours(&self, speaker: &Person, at: (i32,i32)) -> Vec<Rumour { text, about: Option<About>, weight }>`**:
    - Sources, each scored by nearness and weight: events (by year and distance, where the event's text names a site), sagas, `forts`, `beasts_near`, `remains`, the nemesis, and the hero's own deeds.
    - Each rumour can be about a site, a fort, a figure or remains. Hearing it adds the place to `Hero.known` and a `!` on the travel map.
    - The wording comes from phrase tables: three or four openers for each people and each topic. They are original, and chosen by `stories::pick(hash(speaker, topic))`.
  - **Directions**: "Where is …?" lists the known places. The answer gives a bearing and the days of travel ("Four days to the north-east, past the Grey Fells").
  - **The folk**: `Person { name, race, role, civ, home, temper }` is kept in `Visit.people: BTreeMap<u32, Person>`, by creature id.
  - **Greetings** read `Fame`: the famous are known on sight, and the infamous are refused.
- **Steps:**
  1. *Talk and rumours.* Test `talk.rs: a_town_near_a_fallen_fort_talks_of_it` (a fort fallen 3 cells away: one of the first 5 rumours names it, and hearing it marks it known).
  2. *Directions and greetings.* Test `talk.rs: the_famous_are_greeted_by_name`.
- **Writes:** talk.rs, ui/adv_talk.rs, app/adventure.rs (`key_talk`).
- **Reads:** A4 (town folk), A10 (fame).

### A8 Quests  (effort: 2 sessions; risk: medium) — wave 3

- **Design** (game/adv/quests.rs):
  - **Slay, Recover, Clear and Avenge**, as in the architecture. A giver offers at most one quest, and the hero holds at most three.
  - **Completion** is checked after each `act` and on leaving a site.
  - **Rewards** are paid on return to the giver. A quest left undone at death goes into the remains' legend and the A14 hooks.
  - **The quest log** (`j`) gives each quest's target, bearing and days.
  - **Each quest is a saga chapter** when it is taken (Cause or Escalation) and when it is done.
- **Steps:**
  1. *Offer and log.* Test `quests.rs: a_town_offers_to_slay_the_beast_of_the_hills`.
  2. *Completion and reward.* Test `quests.rs: the_relic_comes_home_and_the_giver_pays` and `quests.rs: avenge_the_fort`.
- **Writes:** quests.rs, ui (the quest popup in ui/adv_talk.rs, after A7, or its own `ui/adv_quests.rs`).
- **Reads:** A7.

### A9 Companions  (effort: 2 sessions; risk: medium) — wave 3

- **Design** (game/adv/company.rs):
  - **Hiring** costs coin or needs renown above the person's caution.
  - **`company_tick`** (W0's hook) follows, fights, shoots and flees, as in the architecture.
  - **Orders** (`c`): follow, wait here, attack my target, go home.
  - **Loyalty** falls with wounds, with a companion's death, and with infamous deeds, and rises with shared victories. Below 0 the companion leaves ("Kadol has had enough. She turns for home.").
  - **Carrying** companions between sites and through travel uses `carry_in`/`carry_out`. Travel ticks their bodies too.
  - **A companion who survives the hero** goes home. With A14, they may turn up as a migrant in a later fort of their civ.
- **Steps:**
  1. *Hire and follow.* Test `company.rs: a_hired_companion_follows_across_sites`.
  2. *Fight, flee, leave.* Test `company.rs: a_companion_fights_beside_and_may_quit` (a goblin band: the companion attacks; after two of three companions die, the third leaves at loyalty < 0).
- **Writes:** company.rs, app/adventure.rs (`key_company`).
- **Reads:** A7 (hire through talk).

---

### A12 The sworn enemy  (effort: 2 sessions; risk: medium) — wave 4

- **Design** (game/adv/nemesis.rs):
  - **`Nemesis { figure: u32, name, species, civ, band: u8, grudge: String, at: (i32,i32), last_seen: (i32,i32), since: u64, met: u8 }`.**
  - **Sworn when** the hero kills a goblin general, a figure with kin, or a band's leader, or robs a dark fortress, or flees a megabeast that saw them. The chance of each is a knob.
  - **On the world map** it moves 1-2 cells a day toward `last_seen` (A3's travel loop calls `nemesis_day`). It learns of the hero from rumours in towns the hero passed through, and its band grows each season.
  - **When it reaches the hero's cell**, that is an encounter with the nemesis and its band. Its first meeting is the Wander saga's Turn.
  - **If it wins**, the hero's death gives the nemesis the epithet and the legend. **If the hero wins**, a Grudge saga closes.
- **Test:** `nemesis.rs: the_enemy_follows_and_finds_you` (sworn, then the hero camps in place: within N days it arrives as an encounter).
- **Writes:** nemesis.rs, travel.rs (one call).

### A13 Lairs and foes with a history  (effort: 1.5 sessions; risk: low) — wave 4

- **Design**: lair.rs gains the arrival card with legend lines, the named bones of past heroes with their gear, and "dreaded" marks on the travel map for figures with 3 or more kills. When a fallen fort's killer is still alive somewhere else, the fort's books say where it went.
- **Test:** `lair.rs: the_bones_of_the_hero_who_came_before` (a figure that killed a named figure: bones labelled with that name and an item).
- **Writes:** sitegen/lair.rs, ui/adv_travel.rs (a mark).

### A14 Later forts hear of you  (effort: 1.5 sessions; risk: low) — wave 4

- **Design** (fame.rs `place_remains`, the W0 hook):
  - **Remains on embark**: a fort embarked on a cell with `Remains` finds the bones and gear at the matching spot (clamped to the map), with a look line and a spotlight the first time a dwarf sees them ("the bones of Urist Dragonslayer, her axe beside her").
  - **A surviving companion** joins the next migrant wave of their civ (`Dwarf` pushed into a `wanderers` list in `Overworld`, read by `migrants`). This is one line in the migrants code, which M8 P01's survivors-as-migrants will share. If P01 has landed, use its `wanderers`.
  - **The overseer's notes** mention a famous hero of their civ at embark ("They say Urist Dragonslayer walked these hills").
- **Test:** `fame.rs: a_fort_on_the_spot_finds_her_bones`.
- **Writes:** fame.rs, the migrants line (game.rs `migrants` or wherever it lives then; coordinate with M8).

### A15 Food, drink and trade  (effort: 2 sessions; risk: medium) — wave 4

- **Design**:
  - **Hunger and thirst** tick on the road and in sites (`Dwarf.hunger`/`thirst`, the existing fields). `e` eats or drinks from the pack, and a river cell or a well refills a waterskin.
  - **Hunting** for food: butcher a kill with `g` (yields from the bestiary).
  - **Trade** in towns is a simple shop popup: folk with the smith or brewer role sell from their site's stock, priced with the item values `trade.rs` already uses, for coins.
  - Starving is a cause of death and a saga line.
- **Test:** `travel.rs: a_long_road_without_food_starves` and `town.rs: buy_a_helm_with_coin`.
- **Writes:** travel.rs (one call), sitegen/town.rs (shop), ui/adv_talk.rs (shop popup).

### A16 Tuning the danger  (effort: 2 sessions; two goals of one each; risk: medium) — wave 4

These follow PLAN.md's rules for tuning packages: one goal per agent, a two-hour time box, and a stop after three variants miss. Explore with 10 runs and confirm with 50. Commit each measured step with its batch numbers.

- **Goal T1, the road**: in `--adventure-trial --runs 10`, between 10% and 30% of deaths happen on the road, and a calm-land walk of 5 cells meets 0-2 encounters. Knobs: `travel.*`, `encounter.*`.
- **Goal T2, the site**: a first run against a fallen fort comes home 25-60% of the time, and against a dragon lair under 15%. Knobs: the occupant size in `fallen.*`, the starting kit and the bot's flee threshold. Don't change the combat numbers: the forts' balance depends on them.
- **Writes:** knobs only (`adv.knob` defaults in the modules), and the report.

---

### Work packages and waves

Sessions are agent sessions; "∥" means they can run at the same time.

**Wave 0 (foundations)**: W0-A scaffolding (1.5) ∥ W0-B the turn engine (1).

- W0-B codes against the contract in W0-A and rebases on it before merging. W0-A merges first.
- The coordinator's check is the byte-identical 25-fort batch.
- **Before W0 starts: M2's W0-B must have merged** (glyphs.rs, `LogEntry`, the `Mode` and `Game` struct edits).

**Wave 1 (the slice's body, all new files)**

| Package | Item / steps | Sessions | Main files |
|---|---|---|---|
| A1 | Make a character (1-2) | 1.5 | adv/create.rs, ui/adv_create.rs, app/adventure.rs `key_create` |
| A2 | The local view and keys (1-3) | 2 | ui/adventure.rs, app/adventure.rs `key_local`, window screenshot |
| A3 | Travel (1-3) | 2.5 | adv/travel.rs, ui/adv_travel.rs, app/adventure.rs `key_travel` |
| A4 | Sites when entered (1-4) | 3 | adv/sites.rs, adv/sitegen/{wild,lair,ruin,town}.rs, combat.rs (1 line) |
| A5 | Fallen forts (1-3) | 2 | adv/sitegen/fallen.rs |
| A6 | Fighting as a person (1-3) | 1.5 | adv/fight.rs, combat.rs `strike_at` (new fn) |

Conflicts to manage in wave 1:

- **app/adventure.rs** is shared by A1, A2 and A3. W0-A splits it into one function per view (`key_create`, `key_local`, `key_travel`, `key_end`, `key_talk`, `key_company`), each a stub in its own `impl App` block, so each package edits only its own function. The coordinator merges A2 first, since it owns the shared popup helpers.
- **combat.rs**: A4 adds a line in `creature_tick` and A6 adds a new function. They don't conflict.
- A3 and A5 call A4's `enter`. They code against W0's stub and merge after A4.

**Wave 2 (the slice's soul, ships the slice)**

| Package | Needs | Sessions |
|---|---|---|
| A10 Fame, saga, death, memory (1-4) | A2-A5 (A6 soft) | 2.5 |
| A11 Saving | A4, A5 | 1 |
| A18 The bot and the batch | A3, A4, A6 | 1.5 |

A10 ∥ A11 ∥ A18. A10's Done-when step (4) needs A18's bot, so that one test lands last in the wave, by whichever merges second. After the wave: the first adventure batch (50 runs, about 15 minutes), and the slice is announced in the README.

**Wave 3 (layer 2: people)**

| Package | Needs | Sessions |
|---|---|---|
| A7 Talk and rumours | A4 town, A10 | 2 |
| A8 Quests | A7 | 2 |
| A9 Companions | A7 (hiring) | 2 |

A7 goes first. A8 ∥ A9 follow it, sharing ui/adv_talk.rs: A8 gets its own `ui/adv_quests.rs` to avoid the conflict.

**Wave 4 (layer 3: drama and tuning)**

| Package | Needs | Sessions |
|---|---|---|
| A12 The sworn enemy | A3, A10 | 2 |
| A13 Lairs with a history | A4 | 1.5 |
| A14 Later forts hear of you | A10, A9 soft | 1.5 |
| A15 Food, drink, trade | A3, A7 | 2 |
| A16 Tuning, two goals | all, A18 | 2 |

A12 ∥ A13 ∥ A14 ∥ A15, then A16 alone, against the merged build (tuning rule: "trace with the build that made the batch").

### Interface contracts (build first, then code against them)

All in W0 unless noted.

- `game/adv/mod.rs`: `Adventure`, `Hero`, `Calling`, `Visit`, `SiteKey`, `SiteMemo`, `Fame`, `Deed`, `Quest`, `QuestKind`, `Nemesis`, `AdvStats`, `AdvEnd`, `Adventure::{new, ow, knob, hour}`.
- `game/adv/turn.rs` (W0-B): `Action`, `Target`, `Outcome`, `Game::{adv_step, act, alarm, in_sight}`.
- `game/adv/items.rs` (W0-B): `carry_in`, `carry_out`.
- `game/adv/sites.rs` (stub in W0, built in A4): `enter(adv, key, from) -> Result<(), String>`, `leave(adv) -> SiteKey`, `land(ow, seed, x, y) -> World`.
- `game/adv/travel.rs` (A3): `Adventure::{travel, camp, encounter_chance, roll_encounter}`, `TravelOutcome`, `Encounter`.
- `game/adv/fame.rs` (A10): `Adventure::{deed, hero_died, retire}`, `Game::place_remains` (the W0 stub).
- `game/clock.rs`: `hour`, `is_night`, `light`, `moon`, `time_words`.
- `overworld.rs`: `FortMark`, `Remains`, `Overworld::{forts, remains, fort_at, add_figure}`.
- `sagas.rs`: `SagaKind::Wander`, `Game::saga_wander`; `sagas_fall` writes the `FortMark`.
- `saves.rs`: `SaveKind`, `SaveMeta.kind`, `write_adventure`, `read_adventure`.
- `app.rs`: `Mode::Adventure`, `App.adv`, the `TITLE_*` consts, the game-over `a`. `app/adventure.rs` holds `AdvScreen`, `View` and one `key_<view>` per view. `jobs.rs` holds `JobDone::{Visited, AdvLoaded}`.
- `ui/adventure.rs`: `draw` (and one file per view: `adv_create`, `adv_travel`, `adv_talk`, `adv_end`).
- `combat.rs`: `strike_at` (A6), the `folk_tick` line (A4).
- **For M8 P01**: `FortRecord` should hold or replace `FortMark`, with the same field names where they overlap (`name`, `site` ↔ `x,y`, `founded`, `fate` ↔ `fell`/`killer`, `artifacts`, `save_file`, `peak_pop`). `Occupant` in A5 uses P01's variants (Army, Beast, Horror, Dead, Empty). Whichever lands second adapts.

### Interaction with Milestone 2

M2's wave 0 is being built now (`wave/m2-w0`, at `23f4963`, no commits yet). What M2 owns and when:

| M2 package | Files it edits that G03 also touches | Resolution |
|---|---|---|
| W0-B scaffolding | game.rs (Game fields, `LogEntry`, `step`), app.rs (`Mode` +5), ui.rs (glyphs move out, +5 arms, `look_lines`), report.rs (`overseers`), overseer.rs, new glyphs.rs | **G03 W0 starts after M2 W0-B merges** and rebases on it. G03 uses `glyphs::tile_look`/`unit_glyph` rather than ui.rs's private ones, and `LogKind` for colouring combat lines |
| W0-A facing | overseer.rs, ambitions.rs, layout.rs, blueprint.rs | No overlap |
| P1 O01 | spotlight.rs, personality.rs, app_m2.rs | G03 only calls `spotlight()`. If P1 changes its signature (`spotlight_with`), the old one stays |
| P4 crafts | items.rs, heroes.rs, detail.rs | G03 **calls** `credit_kill`/`earn_epithet` and adds no `ItemKind` (books are labelled crafts, coins are abstract). No edits |
| P5, P7 | app.rs (`Mode::Unit`, `draw` row split), ui.rs `draw` | G03's ui.rs and app.rs edits are one early-return arm each, made in W0. G03 wave 1 does not touch app.rs or ui.rs |
| P9-P11 | game.rs (`find_craft`, dig, `designate`) | No overlap: G03 never edits game.rs after W0's `mod` line and its `embark_prepared` line |
| O09 (wave 2-3) | legends drift and lore, `Overworld.events` | O09 may ask "should legends hide what the fort hasn't heard?" (PLAN.md). G03's rumours are the same idea for an adventurer; share the rule if O09 makes one |

Name clashes to avoid:

- M2 W0-B adds `saga.rs` with its own `SagaKind`. G03 extends the **existing** `sagas.rs::SagaKind` with `Wander`, imports it by path, and never glob-imports `saga::*`.
- M2's `places.rs::Place` and stories.rs's `Place` stay separate. G03 names its things `SiteKey`, `SiteMemo` and `Remains`.

Net: G03's wave 0 is the only part that touches shared files, and it waits for M2's W0-B. G03's waves 1-4 can run **at the same time as M2's waves 1-3**, because they live in `src/game/adv/`, `src/ui/adv*.rs` and `src/app/adventure.rs`. The exceptions are the single combat.rs line and function (M2 doesn't own combat.rs) and A18's one report.rs function, which merges after M2's W0-B.

### Verification per package

- Every package: its listed tests, and `cargo test --release` printing `test result: ok`. No new warnings.
- Any package that adds saved state re-runs `old_saves_load_without_adventure_fields` (W0) with its defaults.
- Packages that touch a fort code path (W0-A, A4's `creature_tick` line, A6's `strike_at`, A14's migrants line): the coordinator's 25-fort batch must stay byte-identical (`--compare` shows 0 forts changed), and `one_year_timing` stays within 10%.
- Adventure batches after waves 2 and 4 (`--adventure-trial --runs 50 --seed 9000 --out batches/adv-w<n>.json`).

### Estimated effort

| Package | Sessions |
|---|---|
| W0-A, W0-B | 2.5 |
| A1 | 1.5 |
| A2 | 2 |
| A3 | 2.5 |
| A4 | 3 |
| A5 | 2 |
| A6 | 1.5 |
| A10 | 2.5 |
| A11 | 1 |
| A18 | 1.5 |
| **Slice (waves 0-2)** | **20** |
| A7 | 2 |
| A8 | 2 |
| A9 | 2 |
| A12 | 2 |
| A13 | 1.5 |
| A14 | 1.5 |
| A15 | 2 |
| A16 | 2 |
| **Total** | **35** |

With 6 agents, the calendar is five waves: the slice in three (about 20 session slots), people in one, drama and tuning in one.

### Risks

- **A fallen fort's save is big and slow to load.** A large fort's JSON can be tens of MB. Load it on the job thread with the loading screen, and keep only one fallen fort whole in memory. An adventure saved inside one is as big as the fort's save. If that proves too slow, a later step can strip what the ruin doesn't need (stockpile settings, the reel, the overseer) on entry.
- **Systems that assume a fort, reached indirectly.** `kill` → `hero_fell` makes "every dwarf mourn"; `credit_kill` texts say "of {fort}"; a creature's AI may path toward a missing depot or tavern (`Visitor`). Setting `fortress_name` to the site's name covers the texts. The engine spike (W0-B) and A4's town test exist to find the rest early. **Mitigation**: `adv_step`'s explicit list means that what isn't called can't misbehave.
- **The hero's ids.** The hero's `Dwarf.id` must stay stable across visits for sagas (keyed by id) and for `Figure` links. Give the hero `id = 1` in every visit; `next_id` starts above it. Companions get 2-4.
- **Turn feel.** Ticks under turns can make a move feel sluggish (a wolf takes two steps per hero step) or unfair. The answer is to show it: the sidebar shows what's in sight, and each interrupt says why. A2's snapshot tests and a play session after wave 1 settle it.
- **Encounter balance is a new axis.** The fort's combat numbers must not change for the adventure's sake. Tuning works on encounter rates and occupant sizes only (A16).
- **No world tick.** The world doesn't change while you travel, apart from your deeds and the nemesis. That is fine for the slice. Note it in the README, and connect it to M4's `world_daily` when it lands.
- **Window mode.** It is free through `ui::draw`, but our tiles must draw crisply. `@`/`☺`, `×` and `!` come from the font; check A2's screenshot at cell sizes 8 and 32.

### Open questions for the user

1. **Who can you play?** The slice is dwarves only, which reuses everything as it is. Should humans, elves (or goblins, as an outlaw) come later as a calling with their own towns, or stay out?
2. **Does an adventure write back to the world for everyone?** As planned, a new fort or adventurer started *from the adventure's death page* (`n`/`a`) inherits its legends. A fort started from the title's "New world" does not, because there is no shared world file until M8's FC6. Is that enough for now, or should G03 build the world file (`saves/<World>.world.json`, about one extra session) so every later game in that world hears of you?
3. **Is a fallen fort's save sacred?** The plan keeps the player's fallen-fort save untouched: the adventurer's looting is remembered only in the adventure. Or should looting it change that save, so the "look upon the ruins" from the game-over page shows the empty pedestal?
4. **When to build it?** PLAN.md puts G03 after M4 and M8. This plan needs neither for its slice, only M2's wave 0. Start the slice once M2 wave 0 merges, in parallel with M2's waves (it lives in its own files), or wait?
