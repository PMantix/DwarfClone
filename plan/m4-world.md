# Implementation plan: Milestone 4

## Milestone 4: the world comes to you (v0.6)

Goal (WISHLIST): the world map is alive. What happens out there reaches the fort, and the fort
reaches back.

### What the code has today (the starting point)

- **The world is frozen once you embark.** `Overworld` (src/overworld.rs) holds civs, sites,
  figures and events. `Overworld::simulate_year` (private) runs only during world generation.
  `Game` keeps the overworld (`Game.overworld`), plus `civ`, `neighbors: Vec<Neighbor>` (worked
  out once in `Game::from_world` and never refreshed) and `start_year`. **The fort is not a site
  on the world map.** `legend()` (trade.rs) adds events to `overworld.events`.
- **Sieges come from a timer.** `combat.rs: maybe_siege` runs each season (`Game::step`'s
  `tick % SEASON == 0` block). It picks a foe from `neighbors` with `at_war_with` (goblins, or
  civs in the home civ's `enemies`), sets `Game.incoming: Option<(u64, Neighbor, u32, String)>`
  for about a day ahead, and `army_arrives` → `lay_siege_of` spawns the army at
  `edge_entry()`. `siege_leader` picks a living figure. No army ever exists on the world map,
  and `Neighbor` has no site, so an army doesn't come from any particular place.
- **Caravans are a timer too.** `trade.rs: caravan_season` sends elves in spring, humans in
  summer and our own dwarves in autumn, unless the home civ is at war with them. Goods come from
  fixed lists in `caravan_goods(race)`. `merchant_died` spills the goods and has no other
  effect. The margin is a flat 10% less the broker's skill (`trade_margin`). The overseer's
  `ov_trade` buys food, then metal, then cloth.
- **Word goes out only.** `word.rs` carries one-way news: the fort's wants reach the
  mountainhome and keep migrants who love those things at home (`word_keeps_away`, used in
  `game.rs: migrants`).
- **Migrants are strangers.** `game.rs: migrants` makes 3–6 new dwarves at an edge, with no
  history, no ties, no pets and no reason for coming.
- **The rest:** expeditions (`expeditions.rs`: Ruin/Raid/Beast; when they lose, members die),
  visitors (`visitors.rs`: bards who sing the fort's own legends, warriors who may join, the
  stranger), the skulker thief (`combat.rs: skulker_think`), cave horrors (a name and look and
  nothing else, `make_cave_horror`), megabeasts from history (`spawn_megabeast`; only dragons
  have a power, `breathes_fire`), ghosts of the unburied (`family.rs: ghosts_tick`), the
  necromancer (`necromancy.rs`, a `Game` field that isn't on the world map), the vault
  (`ruins.rs`) and mysteries.
- **Overseer hooks:** `overseer.rs: ov_daily` (trade, venture, defences), `ov_alert` (reads
  `incoming.0` to raise the bridge 400 ticks before the army arrives), `ambitions.rs: Sense` and
  `ov_venture`, `personality.rs: Trait` (Cautious, Bold, Vain, Pious, Greedy, Sentimental,
  Stubborn, Cheerful) with `voiced()`, and `troubles.rs: Lack`.
- **No in-play world map.** `Mode::WorldMap` is the embark screen only (`app.rs: key_world`,
  `ui.rs: draw_world`).

So much of this milestone is one missing layer: **a world that moves during play** (sites,
parties travelling between them, civs that hold opinions of the fort, news coming in). Build
that once, as foundations, and most items become small consumers of it.

**Order and why**

1. **W25** (UI only, no dependencies) can start on day one in parallel. It also helps anyone
   watching the later work.
2. **Foundations F0** (see "Interface contracts" below): the fort as a world site, `Party` (any
   group travelling the world map) and the daily world tick, `Standing` (each civ's opinion of
   the fort), `Audience` (a demand or offer answered by the player or by the overseer's traits),
   `News`, `Captive`, and figure links. This takes about 3 sessions and everything else in the
   milestone depends on it.
3. **Critical path:** F0 → **B01** (armies and caravans as parties, scouts, roads, the in-play
   atlas) → **B02** (envoys and tribute through Audience) → **B16** (grudges, the parley at the
   gate, leaders) → **B17** (the yearly world pass, wars in play, news) → **B24** (agreements and
   local prices). That is about 22 sessions in series.
4. **Alongside the critical path:**
   - **B15** follows B02 (Standing plus the trade ledger).
   - **B18** follows B01 (routes) and the F0 captives.
   - **B21** follows B18 (rescue missions).
   - **B19** follows B17 (news, figures) for its full form, but its first three steps need
     only Standing.
   - **F07** follows Audience and Party; **B22**'s hunter step reuses F07.
   - **B20** and most of **B23** are local systems that can run from wave 1. B23's "evil with a
     cause" step waits for B17, which puts the necromancer on the world map.
   - **B10** goes last: its starting effects write into Standing, News and the migrant pool.
5. **Duplicate ideas, each built once:**

   | Idea | Listed under | Built in |
   |---|---|---|
   | Parley at the gate | B02 (NOB-16, OVR-01) and B16 (SIE-09) | B16 |
   | Trade that dies off | B02 (NOB-54) and B15 (ECO-07) | B15 |
   | Monster hunters | F07 (OUT-16) and B22 (CRE-07) | F07 |
   | Elves judge forestry / logging angers the wild | B02 (NOB-13), B15 (ECO-05), B20 (AGR-09) | `felled` counter in F0; consumers in each |
   | Liaison negotiations / trade agreements | B02 (NOB-58) and B24 (ECO-21) | B24 |
   | Messengers | B02 (NOB-57) and B24 (ECO-22) | B02, used by B24 |
   | Armies you can watch coming | B16 (SIE-33) and B01 | B01 |
   | Tribute through caravans | B24 (ECO-19) | B24, on top of B02's tribute |

6. **Dependencies on other milestones:**
   - **M1** S03's open step "captives" is the F0 `Captive` store (build it here, tick it there).
     S06 (odds before sending) should land before B18's rescues.
   - **M3:**
     - C06 the council: export bans NOB-43 = B15 ECO-10, and the barony NOB-11 = B02 OVR-09.
       Reuse C06's noble-favour and mandate code; if C06 hasn't landed, B02 and B15 ship the
       civ-level half only.
     - C07 banishment (NOB-03) is the base of B02 OVR-08.
     - C09 tavern performers (SOC-30) overlaps F07's performers: F07 brings visiting troupes,
       C09 the fort's own performers.
     - C02 coming home (MND-37) = B18 OUT-18.
     - C03 grudges from pets (SOC-07) is what AGR-01 sets off.
     - C10 gods: petitions (OVR-11) and temple lore for hunters.
   - **Later milestones build on this one:** M5's B03 "Armies abroad" reuses `Party`, and
     B06 siege AI builds on B16's parley stance. M8's P03 roads reuses B01's roads, P05 "Caravans
     on the world map" (ECO-32) is already done by B01 step 1 (mark it so), and P01 (fallen forts)
     is needed for SOC-17 and CRE-19, which stay as stubs here.

**Release check**

- **Tests:** every item's "Done when" test passes (listed per item below), and
  `cargo test --release` prints `test result: ok`.
- **Batch:** `--autoplay --runs 25 --years 3 --seed 5000 --out batches/m4.json`, then
  `--compare batches/m3-final.json batches/m4.json`. report.rs gains a **World** section, fed by
  a new `Trial.world: WorldNote` field with `#[serde(default)]`. Targets:
  - Forts standing at year 3 within ±2 of M3's. The world should make forts more interesting,
    not much easier or harder.
  - ≥90% of sieges announced at least 2 days ahead and drawn as a party on the atlas.
  - ≥70% of non-goblin sieges carry a stated grievance; goblins state a motive.
  - At least one envoy per fort a year, with answers that differ by trait (Greedy forts refuse
    tribute more than Cautious ones).
  - ≥3 news items heard per fort a year, and a war or peace in play in ≥50% of worlds over three
    years.
  - Caravans skipped because of a bad opinion appear only in forts that harmed merchants or
    envoys.
  - A dwarf taken captive in ≥20% of runs, and at least one rescue across the batch.
  - ≥30% of migrant waves include someone already known in the fort.
- **Performance:** `one_year_timing` within +5% of M3, and save size after three years within
  +10%. Check this with the existing size test in game.rs (the one that prints
  `overworld … events …`).
- **Thing to watch:** `--story` on one run should read as an arc. For example: news that the
  home king and the elves went to war → an elven envoy demands lumber → the Greedy overseer
  refuses ("Counting the cost, it…") → the atlas shows an elven army three days out → the herald
  names the refusal → the siege.

---

### F0 Shared foundations  (effort: 3 sessions; risk: med)
- **Design:** see "Interface contracts" at the end for exact signatures. In short:
  - **The fort as a site.** `Overworld::add_player_site` adds `Site { player: true, kind:
    Fortress, civ: home }`. `Game.home_site: Option<u32>` points at it; older saves get it in
    `after_load`. `neighbors()` and `parent_civ()` skip player sites, and simulated battles
    never target one.
  - **New module `src/game/worldturn.rs`** with `world_daily()`, which moves parties, sights
    them and resolves arrivals, and `world_yearly()`, which refreshes `neighbors` and (in B17)
    rolls history on.
  - **`Party` and `Overworld.parties`.** Armies, caravans, envoys, hunters, refugees, our
    squads, beasts: anything travelling the world map.
  - **New module `src/game/relations.rs`**: `Game.standings: Vec<Standing>`, with `wrong()`,
    `favour()`, `opinion()` and `grudge_line()`, plus yearly drift toward each civ's base
    opinion.
  - **New module `src/game/audience.rs`**: `Game.audiences: Vec<Audience>`. Each audience is
    answered by the player in a popup (`Mode::Audience`) or by `Game::ov_answer(&Overseer,
    &Audience) -> Answer`, a trait table that also writes the note saying why.
  - **New module `src/game/news.rs`**: `Game.news: Vec<News>` and `hear()`.
  - **`Game.captives: Vec<Captive>`** (in news.rs or a small `captives.rs`).
  - **Figure links:** `Dwarf.figure`, `Dwarf.home_site`, `Dwarf.came_for`, and new `Figure`
    fields (deeds, pet, cycle, hoard), all `#[serde(default)]`.
  - **`Game.felled: u32`**: trees felled this year, counted where a chop job finishes.
  - **`Game.world_rng: Rng`**: seeded from the game seed so that world events don't shift the
    fort's own random rolls. This keeps batch comparisons fair.
- **Steps:**
  1. Fort site, `home_site`, `after_load` upgrade and neighbour filtering · overworld.rs,
     game.rs, ruins.rs (`after_load`) · test `the_fort_is_a_site_on_the_world_map` (embark, the
     site exists, `neighbors` exclude it, a save without it loads and gains it).
  2. `Party` with `world_daily` moving along a straight cell line (roads come in B01) · new
     worldturn.rs and one line in `step` · test `a_party_walks_from_its_site_to_the_fort_in_days`.
  3. Standings: `wrong`/`favour`/`opinion`/drift, base opinion by race and war · new
     relations.rs · test `wrongs_sour_a_civ_and_fade_slowly`.
  4. Audience: the store, `Mode::Audience` popup, `ov_answer` trait table, expiry with a default
     answer · new audience.rs, app.rs/ui.rs (one mode, one draw fn) · test
     `the_overseer_answers_by_its_traits` (Greedy refuses a tribute, Cautious with a weak squad
     pays, Sentimental never hands over a dwarf).
  5. News and Captive stores, the `felled` counter and `Dwarf`/`Figure` fields · news.rs,
     dwarf.rs, overworld.rs, jobs where chopping completes · test
     `news_is_heard_once_and_captives_survive_a_save`.
- **Overseer & screen:** no behaviour of its own. `ov_answer` is the one place every later
  demand goes through, so the traits matter everywhere. The status screen's "Neighbours" tab
  (z) lists each civ's opinion and its heaviest grudge.
- **Measure:** none (no behaviour change). Check `one_year_timing`: the daily world tick must
  cost under 0.1 ms.
- **Risks / open questions:**
  - Adding the player site shifts site ids. It doesn't, if the site is appended at the end; old
    saves append it on load.
  - `world_rng` is a new serialized `Rng`. Give it a serde default that derives it from the
    seed.
- **Parallel work:**
  - **Writes:** new relations.rs, audience.rs, news.rs, worldturn.rs; overworld.rs (`Site.player`,
    `Party`, `parties`, `add_player_site`, `neighbors`/`parent_civ` filters).
  - **Hot spots:**
    - game.rs: about 8 new `Game` fields in one block, one `world_daily()` call in `step`, one
      `world_yearly()` call next to `new_year()`, and 4 `mod` lines.
    - dwarf.rs: 3 fields.
    - app.rs: one `Mode::Audience` variant plus `key_audience`.
    - ui.rs: `draw_audience` and one match arm.
  - **Reads:** nothing.
  - **Agent size:** one agent, 2–3 sessions. If it must split: package F0a (site, party,
    worldturn) and package F0b (standings, audience, news, captives) touch disjoint files except
    the `Game` struct block.

### B01 Scouts and armies on the world map  (effort: 3 sessions; risk: med)
- **Design:**
  - **Caravans as parties.** `caravan_season` decides the caravan; `dispatch(PartyKind::Caravan,
    from = nearest living site of the civ, to = home_site)`. Arrival (`world_daily` →
    `party_arrives`) runs today's spawning code, now split into
    `trade.rs: caravan_arrives(party)`.
  - **Armies as parties.** `maybe_siege` becomes `muster`: same choice of foe, but the army
    leaves from that civ's nearest site with `size = siege_strength()` capped by `site.pop / 20`,
    and its leader is chosen at mustering. Travel is 3 cells a day, so a foe 12 cells away takes
    4 days. To keep the S03 rhythm (sieges early in a season), muster runs mid-season for
    arrival in the next.
  - **Scouts.** When a party with `kind == Army` comes within `SCOUT_RANGE` = 9 cells of the
    fort (or on the day it musters, if it's that close), it is `seen`. `Game.incoming` is then
    set from the party as today, so `ov_alert` and the S03 tests keep working, with
    `incoming.0 = party.eta`. Daily lines follow: "The army of X is 2 days away."
  - **Losses thin the site.** When the siege ends, `sites[from].pop -= foes_slain * 15` (SIE-33,
    "kills thin that site's next army").
  - **Roads.** `Overworld::build_roads()` runs once at embark and on load if empty. A\* over
    cells: ocean blocked, mountains cost 4, rivers 2, roads 1 (so roads join up). It links each
    site to the two nearest sites of its civ, and the fort to each neighbour civ's nearest site.
    The result is stored in `Overworld.roads: Vec<Road>`, and parties follow
    `Overworld::route(a, b)`.
  - **The in-play atlas.** `W` in Normal mode opens `Mode::Atlas { x, y }`. It reuses
    `paint_world` and adds an overlay of roads (`·` dim), parties (army `!` in its race colour on
    a dark red background; caravan `$`; envoy `?`; hunters `†`; our squad `»`) and the fort `☼`.
    The side panel lists parties (who, how many, days out) and neighbours with their opinion.
- **Steps:**
  1. Caravans become parties, visible on the atlas (marks ECO-32 of P05 done) · trade.rs
     (`caravan_season` split into decide, dispatch and `caravan_arrives`), worldturn.rs · test
     `a_caravan_travels_the_world_map_before_it_arrives` (the party exists, and the merchants
     spawn only at eta).
  2. Armies muster at a real site; scouts give days of warning with size and leader; arrival
     lays the siege; kills thin the site · combat.rs (`maybe_siege` → `muster`, `army_arrives`
     reads the party), worldturn.rs · done-when test
     `a_siege_is_announced_days_ahead_and_seen_approaching` (warning ≥ 2 days, the party's cell
     gets closer each day, the siege begins at eta). Update the `force_siege` helper to
     fast-forward the party.
  3. Roads between sites · overworld.rs (`build_roads`, `route`) · test
     `roads_join_sites_and_avoid_the_sea`.
  4. The atlas mode with the overlay and its side panel · app.rs, ui.rs (a new `draw_atlas`;
     `paint_world` gains `overlay: &[(i32, i32, char, Color)]`) · test using the `dump` helper:
     the atlas shows the army glyph and "days away".
- **Overseer & screen:**
  - **Overseer:** new `Sense::Threat` (an army seen, weighted by size over days left) raises
    Fortify/Muster. With more than 2 days' warning, `ov_alert` gets a "ready" phase: bring in
    outdoor workers a day early, queue bolts, recall the hunters. Cautious raises the bridge a
    day early; Bold waits for the last 400 ticks.
  - **Screen:** a spotlight when an army is first sighted ("An army on the march: 9 goblins of
    the Vile Gate under Snodub, 4 days away"), then one line a day, and the atlas.
- **Measure:** batch warning days ≥ 2 on average. The first siege kills no more than S03's 2.2
  dwarves, with the bridge up in ≥ 80% of sieges. Forts standing must not jump by more than 3.
  If they do, shorten `SCOUT_RANGE`.
- **Risks / open questions:**
  - Longer warning makes sieges easier ("losing is fun"). Keep about 20% of goblin raids as
    ambushes, seen only one day out (`Party.stealthy`).
  - The first siege's timing moves by the travel time.
  - Glyph widths: stick to single-width characters.
- **Parallel work:**
  - **Writes:** trade.rs `caravan_season`/new `caravan_arrives`; combat.rs `maybe_siege`,
    `army_arrives`, `force_siege` (test); overworld.rs `build_roads`, `route`, `Road`;
    worldturn.rs `party_arrives`; new `src/ui_atlas.rs` (keep it out of ui.rs: one `mod` line and
    one match arm there).
  - **Reads:** F0 `Party`, `dispatch`, `home_site`.
  - **Hot spots:** app.rs: the `Mode::Atlas` variant plus a `key_atlas` fn and one `W` key arm
    in `key_normal`. game.rs: none beyond F0.
  - **Agent size:** one agent, 3 sessions. Steps 3–4 (roads and atlas) can go to a second agent
    once step 1 lands.

### B02 Diplomacy  (effort: 5 sessions; risk: med-high)
- **Design:**
  - **New module `src/game/diplomacy.rs`**, built on Audience and Standing.
  - **Envoys.** Each season, a civ whose opinion has moved (or one never met) may send a
    `Party::Envoy`. On arrival, a `Species::Visitor` with `visitor = ENVOY` walks to the depot
    or meeting zone and opens an `Audience`. Asks: `Tribute(value)`, `Artifact(item)`,
    `Prisoner(creature)`, `Lumber quota`. Offers: `Peace`, `Trade`.
  - **First contact (NOB-56).** `Standing.met == false` gives an introduction, and the envoy's
    tone comes from that civ's history with the home civ (wars in `overworld.events` between
    them).
  - **Consequences.**
    - Paying sets `Standing.truce_until = tick + YEAR` and `favour(+15)`; `muster` skips civs
      under truce.
    - Refusing gives `wrong(RefusedDemand, 20)` and `Standing.muster_bonus += 1`, which doubles
      that civ's muster chance and drops its distance weight, so the army comes sooner.
    - Abusing (imprison or kill the envoy) gives `wrong(AbusedEnvoy, 60)`, enough to cross the
      war line.
  - **Tribute to the homeland (NOB-17).** The autumn home caravan brings a tax collector asking
    10% of the wealth made this year (`wealth() - Standing.wealth_at_last_tax`). Refusing costs
    home opinion, which thins migrants (B19) and angers nobles (C06 hook).
  - **Messengers (NOB-57).** From the atlas, `m` sends a dwarf as `Party::Messenger` to a civ:
    peace, trade or alliance. Success depends on opinion and the messenger's sociability. The
    messenger may not come back.
  - **Prisoners of war (OVR-12).** Invaders caught in cage traps join `Game.prisoners`. An
    Audience lets the overseer ransom them (a standing gain with their civ), release them,
    put them to work, or execute them (a wrong).
  - **The rest:**
    - **Banish (OVR-08):** the C07 banishment hook; the exile becomes a Figure with a grudge,
      who may return with B17's bandits.
    - **Barony (OVR-09):** past a wealth mark, the home envoy offers it; the overseer picks the
      baron or refuses (home opinion −).
    - **Crown's levy (OVR-14):** when the home civ is at war in B17, the levy takes squad 2 as
      an expedition-like party that returns as veterans, or not at all.
    - **Pleas for help (OVR-15):** an ally's site is attacked in B17; send a squad or not.
    - **Petitions (OVR-11):** only the "residency" kind for warriors who stayed; temples and
      guilds wait for C10/B09.
    - **Conduct (OVR-13):** a Reckless or Greedy overseer can choose `Answer::Abuse` in
      `ov_answer`.
- **Steps:**
  1. Envoys with demands and offers, plus first contact · diplomacy.rs, visitors.rs (ENVOY
     walk) · done-when test `paying_tribute_delays_a_siege_and_refusing_brings_it_sooner` (two
     clones of one game, pay vs refuse; mean ticks to the next muster from that civ over 20
     seeds: paid > refused + SEASON).
  2. Treating envoys badly, and the overseer's conduct · diplomacy.rs, audience table · test
     `killing_an_envoy_brings_war`.
  3. Tribute and taxes to the homeland · diplomacy.rs, trade.rs (collector at the home
     caravan) · test `refusing_the_tax_collector_costs_home_favour`.
  4. Messenger missions from the atlas · diplomacy.rs, ui_atlas.rs · test
     `a_messenger_makes_peace_with_a_friendly_civ`.
  5. Prisoners of war (with S03's captives) · diplomacy.rs, traps.rs (caged invader to
     prisoner) · test `a_caged_goblin_is_ransomed_or_executed_by_traits`.
  6. Barony, banishment's exile figure, levy and pleas (each small; can ship together) ·
     diplomacy.rs, positions.rs · test `a_levied_squad_comes_home_with_veterans_or_not`.
- **Overseer & screen:**
  - **Overseer:** every Audience goes through `ov_answer`:
    - Cautious pays when its squad strength is below the expected army.
    - Bold refuses.
    - Greedy refuses tribute unless the danger is high.
    - Vain never gives an artifact.
    - Sentimental never gives a dwarf.
    - Pious honours the elves' tree demands.
    - Stubborn refuses a second demand from the same civ.
    - Reckless style may abuse an envoy.
    Each answer is a `voiced` note.
  - **Screen:** a spotlight card for the envoy with its demand; the player gets
    `Mode::Audience`, which pauses the game; a chronicle line for every answer.
- **Measure:** batch envoys per fort-year ≥ 1; answers broken down by trait in the report; mean
  days from a refusal to the next siege < mean days from tribute to the next siege.
- **Risks / open questions:**
  - Scope: 17 ideas. Cut barony, levy and pleas to step 6, and leave the rest of petitions to
    C10.
  - Decision for the user: may the player imprison or kill envoys at all? DF allows it; it
    makes good stories.
- **Parallel work:**
  - **Writes:** new diplomacy.rs; visitors.rs (`ENVOY` const plus a branch in
    `visitor_think`); traps.rs (caged invader to prisoner); trade.rs (tax collector in
    `caravan_arrives`); combat.rs `muster` (reads `truce_until`/`muster_bonus`, one filter line).
  - **Reads:** F0 Audience/Standing/Party; B01 `muster` and the atlas.
  - **Hot spots:** overseer.rs: none (`ov_answer` lives in audience.rs). game.rs: one
    `diplomacy_season()` call in the season block and the `prisoners` field.
  - **Agent size:** too big for one session. Split into package B02a (steps 1–3) and package
    B02b (steps 4–6), run one after the other.

### F07 Monster hunters and performers  (effort: 3 sessions; risk: low-med)
- **Design:**
  - **Hunters (OUT-16 = CRE-07).** In `maybe_visitor` (visitors.rs), if a CaveHorror or
    megabeast has been on the map for over a season, or a lair is within 10 cells, a
    `Party::Hunter` of 1–3 is dispatched. On arrival they are `Species::Visitor` with
    `visitor = HUNTER`, arms and armour, and fighter stats like a veteran dwarf. They open an
    Audience asking leave to go below; the overseer grants it unless it is Greedy (Greedy asks a
    fee). Then they hunt with their own `hunter_think`: path to the target (`can_reach`), then
    fight through the normal `creature_attack`, with the hostility rule extended so hunters and
    beasts fight. If they die, their body and gear stay where they fell (OUT-16), with a legend
    and a gossip line. If they win, their civ gains `favour(+10)`, they get a legend, and they
    may stay (the `warrior_joins` path).
  - **Performers.** New visitor kinds `DANCER` and `TELLER`. `sing()` is generalised into
    `perform()`: a teller picks an `overworld.events` line (preferring B17 news, figures with
    kills, or the fort's own legends). Listeners get a thought, and the event is
    `hear(news, Source::Visitor)`, so legends of the world become news.
  - **Fame abroad (OUT-12).** When a legend line is made (`legend()`) and a visitor or merchant
    of a civ is on the map, that civ gets `favour(+3)`. The visitor rate in `maybe_visitor`
    scales with average opinion.
  - **Tall tales (OUT-05).** Dwarves back from an expedition tell their deed in the tavern: a
    gossip line, embellished by a random factor, and favour with present visitors' civs.
  - **Questers (OUT-13) and recovery (OUT-06).** A visitor asks after artifacts. A dwarf with
    `sociability > 30` lets slip where one is. The quester opens an Audience for it, or (if
    refused, when evil) steals it with skulker logic. A stolen artifact adds
    `Destination { goal: Goal::Recover(item_label), site }` to `destinations()`.
  - **Treasure trails (OUT-07).** A won Ruin expedition may bring a map fragment that reveals a
    hidden site (`Site.hidden` flag set at worldgen on about 5% of ruins).
  - **Talk of buildings (OUT-28).** Visitors leaving after seeing the monument or engraved halls
    give `favour(+5)` to their civ, plus a `word.rs` Fame entry.
- **Steps:**
  1. Monster hunters · visitors.rs, combat.rs (hostility of HUNTER vs beasts) · done-when test
     `a_hunter_arrives_when_a_cave_horror_lives_and_goes_after_it`.
  2. Performers telling legends · visitors.rs (`perform`) · test
     `a_teller_tells_a_tale_from_the_legends_and_it_becomes_news`.
  3. Fame abroad, tall tales, talk of buildings · visitors.rs, trade.rs `legend()` hook (one
     line), word.rs · test `a_slain_beast_raises_fame_with_present_visitors_civ`.
  4. Questers, recovery, treasure trails · visitors.rs, expeditions.rs (`Goal::Recover`,
    hidden sites) · test `a_quester_steals_an_artifact_and_it_can_be_recovered`.
- **Overseer & screen:**
  - **Overseer:** grants or refuses hunters by `ov_answer`; a Bold overseer sends its own squad
    along (a Kill order on the same target). `ov_venture` weights `Goal::Recover` highly when
    Vain or Sentimental.
  - **Screen:** an arrival card for the hunters, a card for a hunter's death ("The bones of X lie
    in the caverns"), and lines in the tavern.
- **Measure:** at least one hunter in about 30% of forts with a cave horror; the hunters' death
  rate is visible in the report's killers; fame is shown per civ.
- **Risks / open questions:** hunters killing the horror too easily takes away a fort's story.
  Cap them at the strength of a "trained veteran".
- **Parallel work:**
  - **Writes:** visitors.rs (most of it); expeditions.rs (`Goal::Recover`, `destinations`);
    word.rs (Fame).
  - **Reads:** F0 Audience/Party/News.
  - **Hot spots:** combat.rs: one hostility line in `creature_think`. trade.rs `legend()`: a
    single `self.fame_from_legend(&text)` call.
  - **Conflict:** expeditions.rs is also changed by B18, so serialize the expeditions.rs part
    after B18, or put `Goal::Recover` in B18's enum change.
  - **Agent size:** one agent, 2–3 sessions.

### W25 Getting around big maps  (effort: 3 sessions; risk: low)
- **Design (UI only):**
  - **Mini-map.** New `src/ui_minimap.rs` draws the fort mini-map: the current z-level squeezed
    to fit, with walls, floor, water, dwarves and hostiles (red), and the view outlined. It is
    toggled by `F2` (`App.minimap: bool`) and cached per `(z, dig_version, w, h)`; add a
    `Game::dig_version()` getter. Clicks register `Target::MiniMap(rect)` via `add_target`, and
    a click centres `view_x`/`view_y`.
  - **Camera spots.** `Game.camera_spots: [Option<Pos>; 9]` (serde default). Ctrl+1–9 or Alt+1–9
    saves (terminals differ, so accept both), and 1–9 in Normal mode jumps.
  - **Jumps.** `Home` jumps to the surface and `End` to the deepest dug level (UI-31).
  - **World names and favourite seeds.** A typed world name in `WorldSetup` hashes to the seed;
    `f` on the world map stars the seed in `save_dir/favourite_seeds.txt`.
  - **Places list (UI-21).** A new Status tab "Places": zones, piles and workshops. Enter jumps
    to one.
  - **Map notes (UI-17).** `Game.map_notes: Vec<(Pos, char, String)>`, placed with `'` in Look.
    The overseer adds notes too (gate, shelter, station).
  - **Depth (UI-19).** Open tiles show the level below, dimmed.
  - **Quick hops (UI-34).** In Look, Enter on an item goes to whoever holds it.
  - **Stocks search (UI-28).** `/` filters the stocks list.
- **Steps:**
  1. Mini-map with click-to-jump · new ui_minimap.rs, app.rs mouse handling · done-when test
     `clicking_the_minimap_moves_the_view` (App with game for tests; synthesise a click inside
     the minimap rect).
  2. Camera spots and surface/deepest jumps · app.rs, game.rs (one field) · test
     `saved_camera_spots_jump_back`.
  3. World names and favourite seeds · app.rs `key_world_setup`, ui.rs `draw_world_setup` · test
     `a_typed_world_name_gives_the_same_world`.
  4. Places tab, map notes (including the overseer's), depth shading, quick hops, stocks search ·
     ui.rs (`status_lines` tab, `tile_look` depth), app.rs · tests
     `map_notes_persist_and_show`, `stock_search_filters`.
- **Overseer & screen:** the overseer pins notes on what it builds (gate, shelter, station), so
  watchers see its plan; spectator mode (O05) can jump between camera spots.
- **Measure:** no batch needed. Check the frame time on a 320×256 map with the minimap on stays
  under +1 ms (the F3 overlay).
- **Risks / open questions:**
  - Ctrl+digit doesn't reach many terminals, hence Alt as well.
  - `F2` must not clash with `F3` (the performance overlay).
- **Parallel work:**
  - **Writes:** new ui_minimap.rs; app.rs `key_normal` (digit, Home/End, F2 arms), `key_world_setup`.
  - **Reads:** nothing.
  - **Hot spots:** ui.rs: the `draw_map` call site (one line), the `status_lines` Places tab,
    and `tile_look` for depth (step 4: keep it last to avoid clashes with B01's atlas). game.rs:
    2 fields.
  - **Agent size:** one agent, 2–3 sessions. Fully parallel with everything (only app.rs key
    arms overlap with B01's `W` arm: trivial merge).

### B15 The caravan remembers  (effort: 3 sessions; risk: med)
- **Design:** caravans read and write `Standing`.
  - **`Standing.ledger: TradeLedger`** keeps the merchants' profit per visit, what we sold last
    year by kind, and whether they were robbed.
  - **Margin and haggling.** `trade_margin()` gains opinion: `-opinion / 10`, clamped to
    −5..+20. `trade()`'s error gives the reason: the broker and their traits, the merchants'
    mood, how short the offer falls (ECO-06).
  - **Goods (ECO-03).** `caravan_goods(race)` → `caravan_goods(civ, from_site)`. The base list,
    plus extra units of whatever `self.troubles` lacks (Lack::Food → meals, Ore/Bars → bars,
    Wood → logs), more when the opinion is high. `ov_trade` ranks goods by the overseer's open
    troubles first.
  - **Starving dwarves raid the depot (ECO-01).** A dwarf at a critical need with no food or
    drink reachable takes one from `caravan.goods` at the depot (needs.rs, where food is
    sought). If goods worth the price are in the depot, the merchants take them; otherwise
    `wrong(RobbedCaravan, value / 5)`.
  - **Harming merchants (ECO-02).** `merchant_died` gets a killer: new `Creature.last_hit_by:
    Option<Attacker>`, set in the strike functions. A dwarf or trap gives
    `wrong(HarmedMerchant, 40)`, and a destroyed caravan gives 80, past the war line.
  - **Trade that dies off (ECO-07 = NOB-54).** After two visits without profit, the next
    caravan comes with half the goods, and after three it skips a year.
  - **Wary caravans (ECO-08).** At dispatch, if opinion < −20, or corpses or hostiles lie within
    12 tiles of the edge-to-depot path, the caravan turns back: "The merchants of X saw the
    bodies on the road and turned back."
  - **Neighbours judge your conduct (ECO-05).** Elves count `felled` and wooden goods offered
    (they refuse them, with `wrong(FelledTrees)` when `felled > 40`); humans and dwarves note
    executions (justice hook).
  - **Export bans (ECO-10).** `Standing.bans: Vec<ItemKind>` (the home civ bans selling weapons
    to its enemies, and C06 nobles add their own). `mark_for_trade` skips banned goods unless
    the overseer is Greedy and willing to take the hit.
  - **Nobility drives trade (ECO-09).** Each noble rank adds 25% to the home caravan's goods; a
    barony widens `neighbors` from 30 to 45 cells.
- **Steps:**
  1. Goods lean to troubles and the overseer buys against them · trade.rs, overseer.rs
     `ov_trade` rank closure · test `a_hungry_fort_is_brought_food`.
  2. Haggling explained (margin from opinion and a reason string) · trade.rs · test
     `a_soured_merchant_wants_more_and_says_why`.
  3. Harming merchants and starving raids (`last_hit_by`) · trade.rs, creature.rs (one field),
     combat.rs (set on strike), needs.rs · done-when test
     `robbing_a_caravan_sours_the_civ_and_its_next_caravan_does_not_come`.
  4. Trade dying off and wary caravans · trade.rs · test
     `unprofitable_visits_thin_the_caravans`.
  5. Conduct judged, export bans, nobility · trade.rs, justice.rs (one hook), positions.rs · test
     `elves_refuse_wood_from_a_fort_that_fells_forests`.
- **Overseer & screen:**
  - **Overseer:** buys against troubles; Greedy may sell banned goods; the Reckless style may
    seize a weak caravan (OVR-13, a rare roll) and write a note.
  - **Screen:** the trade screen shows the margin breakdown and the merchants' mood; a message
    and chronicle line when a caravan refuses to come.
- **Measure:** batch caravans came/skipped/robbed per civ; skipped only in forts with wrongs;
  overseer trade deals per year not below M3's.
- **Risks / open questions:** the opinion-based margin plus the profit ledger could starve a
  poor fort of trade. Floor: the home caravan always comes unless the home civ has fallen.
- **Parallel work:**
  - **Writes:** trade.rs (most); creature.rs `last_hit_by`; combat.rs `strike` (one line);
    needs.rs (food search fallback); overseer.rs `ov_trade` (that function only).
  - **Reads:** F0 Standing; B01 `caravan_arrives`/dispatch split; B02 `felled` usage.
  - **Hot spots:** overseer.rs: only `ov_trade`. combat.rs: one line.
  - **Agent size:** one agent, 2–3 sessions. Runs in the same wave as B16 if B16 stays out of
    trade.rs.

### B16 Enemies with faces  (effort: 4 sessions; risk: high)
- **Design:**
  - **The leader (SIE-06).** `siege_leader` is chosen at muster (B01) and given a profile:
    `deeds` (events naming them from `overworld.events`, at most 3), a `motive` (the civ's
    `grudge_line()`, else by race: goblins "for plunder" or "for sport", humans "for the
    crown"), and a looks line. The warning card and `Mode::Look` on the general show it.
  - **Grudges with reasons (SIE-05).** `muster` weights foes by `distance weight +
    grievance weight` (from Standing). Raids (B18), ambushed caravans, felled sacred trees and
    a broken siege all add grievances. The herald's line at `lay_siege_of` uses
    `grudge_line(civ)`: "They come for the merchants slain at your depot in 253." An old wrong
    adds `+weight/20` soldiers.
  - **Beasts of war and pets (SIE-36).** `lay_siege_of` adds 0–2 war beasts by race: goblins
    bring a new `Species::Troll` and giant spiders, humans war dogs, elves bears. The leader may
    have a named pet: a `Creature.pet_of` reuse with `general` set and the label "Gutrend, war
    troll of Snodub". It stays near the leader; when the leader dies it flees or rages.
  - **Parley at the gate (SIE-09 = NOB-16 = OVR-01).** About 40% of non-goblin armies, and 20%
    of goblin armies, halt at the edge: new `Creature.parley_until: u64`, set on all siege
    creatures, holding them in place. The herald opens an Audience: Ask Artifact (the most
    valuable artifact), Tribute, the Criminal (from `crimes`), or a Noble.
    - Accept: the army leaves, truce for 2 years, legend.
    - Refuse or time out: assault.
    - Mid-siege white flag: when attackers are below half and the leader lives, a 30% chance
      they offer to withdraw for a ransom (again an Audience).
  - **Carried off alive (SIE-24).** A downed dwarf (unconscious or crippled, from the body
    model) near a goblin may be picked up: new `Creature.carrying: Option<u32>`. If that goblin
    leaves the map, the dwarf becomes a `Captive { civ, site: party.from }`. Rescue is B18.
  - **Evil armies with character (SIE-32).** Goblin generals may ride a beast (the label "astride
    a giant bat", ×1.5 body scale, faster). Rarely, when two civs at war with each other both
    muster on the fort the same season, both arrive; new `Creature.side: Option<u32>` makes the
    armies fight each other (`creature_think` targets differing sides).
  - **Watched armies (SIE-33)** come from B01.
- **Steps:**
  1. Leader profile and motive on the warning card · combat.rs (`siege_leader`, `muster`),
     spotlight text · test `the_warning_card_names_the_leaders_deeds`.
  2. Grudges with reasons drive who comes and what the herald says · combat.rs `muster`,
     relations.rs `grudge_line` · done-when test
     `a_siege_leader_names_the_wrong_and_their_army_was_on_the_map` (wrong a civ, force a
     muster: the party exists on the map, and the siege message contains the grievance).
  3. War beasts and leaders' pets · combat.rs `lay_siege_of`, creature.rs (Troll) · test
     `a_general_brings_a_named_war_beast`.
  4. Parley and the white flag · combat.rs (`creature_tick` honours `parley_until`), audience
     kinds · test `paying_at_the_parley_sends_the_army_home`.
  5. Carried off alive · combat.rs (carry and leave), news.rs Captive · test
     `a_downed_dwarf_is_carried_off_and_becomes_a_captive`.
  6. Mounted generals and rival armies · combat.rs, creature.rs `side` · test
     `two_enemy_armies_fight_each_other_at_the_gate`.
- **Overseer & screen:**
  - **Overseer:** parleys go through `ov_answer` (Cautious pays when outmatched; Vain never
    gives an artifact; Sentimental never gives a dwarf, though a criminal is fine; Stubborn
    refuses). A grievance the overseer caused (OVR-13) adds a note "It has made enemies of the
    X." A captive taken raises `Sense::Captive` for B18.
  - **Screen:** cards for the warning (leader profile), the herald (grievance), the parley
    choice, "carried off", and the white flag.
- **Measure:**
  - ≥70% of non-goblin sieges with a grievance line.
  - Parley offered in about 30% of sieges and paid in 10–50% depending on style.
  - Captives taken in ≥20% of goblin-siege forts.
  - Dwarves lost per siege within ±20% of M3. Being carried off replaces a death, so don't
    count it twice.
- **Risks / open questions:**
  - `creature_think` and `creature_tick` are the hottest functions in combat.rs and the most
    contended in parallel work (see Hot-spot discipline).
  - Rival armies need faction-aware hostility everywhere `hostile()` is used.
  - Carrying adds path searches while fleeing; reuse `leaving` paths.
- **Parallel work:**
  - **Writes:** combat.rs `muster`, `siege_leader`, `lay_siege_of`, `creature_tick` (parley
    gate), `creature_think` (side and carry); creature.rs (Troll, `parley_until`, `carrying`,
    `side`).
  - **Reads:** F0 Standing/Audience/Captive; B01 muster/party; B02 truce fields.
  - **Hot spots:** combat.rs is the conflict zone. Do the "split `creature_think`" refactor
    first, and B21 must not edit the same functions in the same wave.
  - **Agent size:** too big for one session. Split into package B16a (steps 1–3: muster,
    leader, beasts) and package B16b (steps 4–6: parley, carry, sides). B16b runs after B16a.

### B17 The world turns  (effort: 5 sessions; risk: high)
- **Design:**
  - **The yearly pass (WLD-05).** `world_yearly()` makes `Overworld::simulate_year` `pub(crate)`
    and calls it with `world_rng` for `year = start_year + tick / YEAR`. Changes:
    - `battle()` skips `site.player`.
    - The mountainhome rule stays.
    - It returns the index of the first new event, so the new events can be gathered.
    - Each new event is turned into a `News` with a `NewsKind`, by parsing at creation: change
      the `log` calls in `simulate_year`/`battle` to `log_kind(year, text, kind)`, adding
      `Event.kind: Option<NewsKind>` (serde default).
    Then `neighbors` is refreshed. If the home civ is gone, there are no more home caravans or
    taxes, and migrants become refugees (B19).
  - **Wars in play (WLD-06).**
    - The home civ's wars follow the simulated diplomacy.
    - The fort's own wars: `Standing.opinion < -60` → `Standing.war = true` ("The X declare war
      on <fort>"). `at_war_with` checks it.
    - Peace: through messengers, tribute, or opinion recovering above −20 for a year.
    - The world's diplomacy roll takes the home civ's grievances from the fort into account
      (the fort drags its civ into war: OVR-13).
  - **News and rumours (WLD-07).** `News` arrives:
    - with merchants (on `caravan_arrives`: 2–4 news items about their region since the last
      visit),
    - with migrants (1 each, B19),
    - with visitors (F07),
    - by the deep roads (home news every season).
    About 10% are false (`true_ = false`: a dragon seen that isn't there, a dead king who
    lives). They are shown in a "News" tab of the status screen and a yearly digest card.
    Acting on news:
    - a beast seen within 10 cells, or an enemy massing, adds a Fortify lean;
    - war between the home civ and a civ stops that civ's caravans (already via enemies);
    - a dwarf whose `home_site` fell gets a grief thought.
  - **Towns fall and rise (WLD-19).** `battle()` conquest fates: raze, plunder (`site.plundered`),
    tributary (`site.overlord`), puppet, annex. Ruins near a living civ resettle at 5% a year.
  - **Alliances against doom (WLD-20).** When a megabeast has razed ≥2 sites, or the
    necromancer is known (now `Overworld` gets the necromancer as a Figure plus a Tower site,
    migrated from `Game.necromancer` in `after_load`), nearby civs form an `Alliance`. An allied
    `Party::Army` marches past the fort (visible on the atlas) and may fight undead at the
    edge.
  - **Refugees and bandit camps (WLD-21).** A razed site sends `Party::Refugees` to the nearest
    safe site, or the fort if it is near (B19 SOC-46). Some settle as a `SiteKind::Camp` in a
    ruin and raid caravans (B15 skip: "bandits on the road") and the fort (small Raider
    sieges).
  - **WLD-32 (XL), cut to scope:** named figures linked to the fort (migrants' kin, captives,
    leaders) age, marry and die in the yearly pass. The rest of the world stays abstract.
- **Steps:**
  1. Yearly pass in play with typed events, refreshed neighbours, the player site protected,
     and a news digest · overworld.rs (`simulate_year` pub(crate), `log_kind`, `battle` skip),
     worldturn.rs · done-when test
     `after_three_years_the_legends_show_wars_and_deaths_elsewhere_that_the_fort_heard_about`
     (events with year > start_year exist, and each has a matching heard `News`).
  2. Wars and peace in play, including the fort's own wars · relations.rs, combat.rs
     `at_war_with` (one condition) · test `a_civ_soured_enough_declares_war_on_the_fort`.
  3. News carried by merchants, migrants, visitors and the deep roads, true and false, with
     reactions · news.rs, trade.rs (one call), word.rs (deep roads) · test
     `merchants_bring_news_and_some_of_it_is_false`.
  4. Towns fall and rise · overworld.rs `battle`, `simulate_year` · test
     `a_razed_town_is_resettled_in_time`.
  5. Refugees and bandit camps · overworld.rs, worldturn.rs · test
     `a_bandit_camp_appears_in_a_ruin_and_raids_the_road`.
  6. Alliances and the necromancer on the map · overworld.rs, necromancy.rs migration · test
     `an_alliance_marches_against_the_necromancer`.
  7. Linked figures live on (WLD-32 scoped) · overworld.rs · test
     `a_migrants_brother_back_home_marries_and_dies_in_the_legends`.
- **Overseer & screen:**
  - **Overseer:** new `Sense::Rumour` (danger in the news) leans Fortify and Muster; war
    declared on the fort leans Muster; the Stubborn trait discounts rumours, and Cautious
    believes false ones too.
  - **Screen:** a "News from the world" card at New Year (top 4 items); the status News tab;
    the atlas shows sites changing hands (colours) and ruins (`×`); a chronicle line.
- **Measure:** batch world changes per run (wars, peace, falls), ≥3 news per fort-year, and
  `one_year_timing` within +2% (simulate_year is cheap, but the events vector grows: cap legends
  shown, or trim generation-era events past 2,000 into a summary).
- **Risks / open questions:**
  - The home civ could be wiped out in play: decide whether that is allowed (losing is fun:
    yes, but rarely).
  - Determinism: always use `world_rng`, never `self.rng`, inside the world pass.
  - Save growth from events: measure it.
- **Parallel work:**
  - **Writes:** overworld.rs (`simulate_year`, `battle`, `log_kind`, `Event.kind`, `Alliance`,
    `SiteKind::Camp/Tower`); worldturn.rs; news.rs; necromancy.rs (migration only).
  - **Reads:** F0 everything; B01 parties and routes; B02 messengers for peace.
  - **Hot spots:**
    - overworld.rs `simulate_year`/`battle` are owned by B17 alone in its wave.
    - ui.rs: a News tab in `status_lines`; or better, put tab content in a new
      `src/ui_world.rs` that `status_lines` calls.
    - combat.rs `at_war_with`: one line.
  - **Agent size:** too big for one session. Split into package B17a (steps 1–3) and package
    B17b (steps 4–7). B17b can run in parallel with B24/B19 because it is all in overworld.rs.

### B18 Missions that matter  (effort: 3 sessions; risk: med)
- **Design:**
  - **Expeditions travel (OUT-22).** `Expedition` gets a `party: u32`, and the squad travels as
    `Party::Squad` along `route()`. Days come from the real route length (`Destination::days`
    → `route.len() / 4`). The route is drawn on the atlas while away and kept after the return.
  - **Captives (OUT-01).** In `expedition_returns`, when beaten by a goblin, human or dwarf
    site, each would-be death has a 40% chance to become a `Captive { civ, site, since }`
    instead (beasts kill). The captive is named in the report, and family get "worried for X,
    held at Y" (a recurring thought while held).
  - **Rescue missions (OUT-02).** `destinations()` adds `Goal::Rescue(dwarf_id)` at the
    captive's site, with danger = site danger + 1. Success returns the dwarf "changed": +stress
    memory, possibly an epithet ("the Unbroken"), and a relation shift toward the rescuers.
    Captives held over 2 years may turn (goblin captives become soldiers: B21 uses this).
  - **Retaliation (OUT-03).** A raid adds `wrong(RaidedSite(site), 30)`, so the next muster
    from that civ is likelier and its herald says "for the raid on X" (B16).
  - **Pillage depletion (OUT-21).** `Site.plundered: u8` goes +1 per successful raid and recovers
    1 a year. Loot is one item per surviving raider times `(3 - plundered) / 3`, and danger is
    lower.
  - **Coming home (OUT-18 = C02 MND-37).** On return, spouses and friends get reunion thoughts;
    a dwarf whose kin died while away grieves now (scan `annals` deaths since `depart`); shock
    if fort wealth fell by over 30%.
- **Steps:**
  1. Expeditions as parties with real routes, drawn on the atlas · expeditions.rs, worldturn.rs ·
     test `an_expedition_walks_its_route_and_returns_after_its_length`.
  2. Captives and rescue · expeditions.rs, news.rs · done-when test
     `a_beaten_expedition_leaves_a_captive_and_a_later_one_brings_them_home`.
  3. Retaliation and pillage depletion · expeditions.rs, overworld.rs (`plundered`) · test
     `raiding_a_site_twice_brings_less_loot_and_a_counter_raid`.
  4. Coming home · expeditions.rs · test `a_returning_soldier_grieves_a_spouse_who_died_while_away`.
- **Overseer & screen:**
  - **Overseer:** new `Sense::Captive` (Sentimental ×2, Bold ×1.5) raises Venture, and
    `ov_venture` scores `Goal::Rescue` above Raid. Cautious waits for odds ≥ 0.6 (S06).
  - **Screen:** a card for captives ("X was taken alive to Y"), the rescue's return, and the
    route on the atlas; the unit sheet shows "held at Y since Z".
- **Measure:** batch captives taken and rescued; expedition deaths not above S06's < 5% of
  deaths (captives aren't deaths).
- **Risks / open questions:** removing `Dwarf` into `Captive` must release jobs, items and squad
  membership, the same way `send_expedition` does. Reuse that code (factor out
  `take_dwarf_off_map(i) -> Dwarf`).
- **Parallel work:**
  - **Writes:** expeditions.rs (owned), ambitions.rs `ov_venture` (score closure only),
    overworld.rs (`Site.plundered` field only).
  - **Reads:** F0 Captive, Party, Standing; B01 `route`.
  - **Hot spots:** ambitions.rs `ov_venture`, plus one `Sense` variant (append at the end of the
    enum and of `Sense::ALL`).
  - **Agent size:** one agent, 2–3 sessions. It owns expeditions.rs in its wave, so F07 step 4
    comes after it.

### B19 Faces at the gate  (effort: 3 sessions; risk: med)
- **Design:**
  - **New module `src/game/migrants.rs`.** Move `migrants()` out of game.rs first (Hot-spot
    discipline).
  - **Death drives migrants away (SOC-18).** Wave size × `(1 - deaths_last_year / pop).max(0.3)`,
    with dead nobles −1 each and home opinion < 0 → −1. The message says why.
  - **Turning migrants away (SOC-44).** If food or beds are short, an Audience: accept, or turn
    the wave away. Refusing costs `favour(home, -10)`.
  - **Ask for workers (SOC-45).** `Game.requested_labors: Vec<Labor>`, set by the player at the
    home caravan's liaison, or by the overseer from `troubles` (Lack::Ore → Mining/Smelting).
    The next wave's specialist pool is weighted by it.
  - **Migrants with a past (SOC-16).** Founders get `home_site` = the parent civ's nearest site
    at embark. Migrants come from a site of the home civ, weighted by distance. A migrant from a
    site some founder shares gets a relation: a friend (`relations` +40), or kin (shared
    parents or a sibling note: "the sister of Urist"). Each migrant has a figure in the
    overworld (`add_figure` made pub(crate)) and `came_for`, chosen from (in order):
    - news of their site's fall ("fleeing the sack of X"),
    - fame and wealth ("heard of the gold of <fort>"),
    - kin in the fort ("following their brother"),
    - the default ("sent by the king").
  - **Survivors of fallen forts (SOC-17):** a stub until P01. If any `site.player && ruined`,
    use it as a home site.
  - **Migrants with baggage (SOC-53).** 30% bring a cat or dog (`spawn_tame` + `adopt`), couples
    may bring an infant, and specialists bring a tool or goods of their trade.
  - **Persecuted kin (SOC-46).** If B17 news has a human town of a civ at war with the home civ,
    or a purge event, a refugee wave of 3–5 arrives with `came_for = "fleeing persecution in
    X"`, and a small human `Party::Army` (2–3) of pursuers 2 days behind.
- **Steps:**
  1. Move migrants into migrants.rs (no behaviour change; existing tests pass) · game.rs,
     migrants.rs.
  2. Death drives them away, turning waves away, asking for workers · migrants.rs, audience ·
     test `a_deadly_year_thins_the_next_wave_and_says_why`.
  3. Migrants with a past and someone who knows them · migrants.rs, overworld.rs `add_figure`
     visibility · done-when test
     `a_migrant_arrives_with_a_reason_and_someone_already_knows_them`.
  4. Baggage, persecuted kin (and the SOC-17 stub) · migrants.rs · test
     `refugees_arrive_with_pursuers_behind_them`.
- **Overseer & screen:**
  - **Overseer:** decides to turn away by traits (Cautious and Greedy turn away when food is
    under 2 days per head; Sentimental never does); requests workers from its troubles.
  - **Screen:** the arrival message lists one face with their reason ("Kel Ironmouth, fleeing
    the sack of Ostuk; Urist knows him from home"); the unit sheet shows "came for" and home
    site; a chronicle line.
- **Measure:** batch migrants per wave (should drop after deadly years); ≥30% of waves with a
  known face.
- **Risks / open questions:** relations from the start may set off C03 feuds early. That's fine
  and good drama.
- **Parallel work:**
  - **Writes:** new migrants.rs (owned); dwarf.rs `new_dwarf` (no change; set fields after it);
    overworld.rs `add_figure` → pub(crate).
  - **Reads:** F0 News/Standing/Audience/figure fields; B17 news kinds for step 4.
  - **Hot spots:** game.rs: removal of `fn migrants` (step 1 ships alone and early, ideally in
    the pre-wave refactor), plus the call site stays as is.
  - **Agent size:** one agent, 2–3 sessions. Steps 1–3 need only F0; step 4 waits for B17a.

### B20 Pests and predators  (effort: 4 sessions; risk: med)
- **Design:**
  - **New module `src/game/pests.rs`.** Abstract colonies, not creatures, for speed.
  - **Vermin (AGR-04).** `Game.vermin: Vec<Colony { zone: u32, size: u8, since: u64 }>`.
    - **Daily** (`pests_daily`): a food stockpile with over 30 food items, no cat within 10
      tiles and no containers covering the food may start a colony (chance by size). Colonies
      eat 1 food per `size` a day (food in barrels or bins is safe), and grow.
    - **Cats:** each cat (pet or tame) whose owner works near, or that wanders into the zone,
      knocks off size (the `CAT_ANTICS` "caught a rat" line becomes real).
    - **Farms:** burrowers eat seeds in fields.
  - **Pets eat pets (AGR-01).** A dog with no meat stocks near another dwarf's cat may kill it
    (`beast_strikes_beast` exists in animals.rs). Owners get relations −50 and a grudge (C03).
  - **Cats fish (AGR-14).** A cat near deep water now and then spawns a Fish item.
  - **Fish run out (AGR-10).** `Game.fish_stock: Vec<u16>` per 16×16-column cell of the map.
    `do_fish` takes from it and `find_fishing` prefers stocked banks; it regrows each season. A
    warning at <20%, and alligators and bears come closer (wildlife spawn bias toward water).
  - **Livestock raids (AGR-02).** At night, pastured animals outside with no dog nearby may be
    rustled by a kobold or bandit skulker (B21 sneak with goal Animal).
  - **Wildlife populations (AGR-18).** `Game.wild: BTreeMap<String, i32>` per species from the
    biome's `fauna()`. Hunting and kills decrement, `wildlife()` spawns weighted by population,
    predators take prey abstractly, and numbers regrow yearly.
  - **Logging angers the wild (AGR-09).** On savage land (`savagery > 0.6`), `felled` past 30 a
    year sends an "enraged" pack (wolves or bears, labelled "enraged", aggro high) and tells the
    elves (B15).
- **Steps:**
  1. Vermin colonies and cats · pests.rs, a hook in needs or haul for containers · done-when test
     `a_hoarded_food_store_draws_vermin_and_a_fort_with_cats_loses_less`.
  2. Pets eat pets, and cats fish · pests.rs, pets.rs (one call) · test
     `a_hungry_dog_kills_a_cat_and_the_owners_fall_out`.
  3. Fish stocks · farming.rs `find_fishing`/`do_fish` · test `overfishing_empties_a_bank_and_it_recovers`.
  4. Wildlife populations and logging anger · combat.rs `wildlife` (spawn weighting), pests.rs ·
     test `felling_a_savage_forest_brings_enraged_beasts`.
  5. Livestock raids (after B21's sneak goals) · pests.rs · test
     `an_unguarded_pasture_loses_a_goat_at_night`.
- **Overseer & screen:**
  - **Overseer:** new `Lack::Vermin` trouble; the fix is to adopt or breed cats and put food in
    barrels (a work order). Fish running low lowers the fishing labor; Sentimental forbids
    culling.
  - **Screen:** "Rats are in the food stockpile!", a vermin mark on the pile in Look, and
    fish-stock warnings.
- **Measure:** batch food lost to vermin per fort-year (report line); starvation deaths not up
  more than 1 percentage point vs M3.
- **Risks / open questions:** vermin plus fish depletion could re-open M1's food troubles (S05).
  Tune colony growth after a batch.
- **Parallel work:**
  - **Writes:** new pests.rs (owned); farming.rs `find_fishing`/`do_fish`; combat.rs
    `wildlife()` (that function only); troubles.rs (one Lack variant plus fix arm).
  - **Reads:** F0 `felled` (or count it itself if F0 slips).
  - **Hot spots:** game.rs: one `pests_daily()` call in the daily block, 3 fields. troubles.rs:
    append-only.
  - **Agent size:** one agent, 3 sessions. Wave 1 eligible (it needs nothing but `felled`).

### B21 Snatchers and thieves  (effort: 3 sessions; risk: med)
- **Design:**
  - **Generalise the skulker into sneaking with a goal.** New `Creature.sneak:
    Option<SneakGoal>` with `SneakGoal::{Item, Child, Animal, Lever, Corpse}`. `skulker_think`
    becomes `sneak_think`, in a new `src/game/sneaks.rs`. Dogs reveal sneakers within 3 tiles
    (a new check in `sneak_think`).
  - **Baby snatchers (CRE-04).** In winter or at night, a goblin snatcher (`Species::Goblin`,
    sneaking, goal Child) takes a child not within 4 tiles of an adult, then leaves. The child
    becomes a `Captive { civ, site: nearest goblin site }`: spotlight, family grief, and rescue
    via B18. Years later they may come back in a siege: B16's `lay_siege_of` spawns one turned
    captive with the label "X, once of <fort>". Siblings and parents who see them get a shock.
  - **Gremlins (CRE-11).** New `Species::Gremlin`: always sneaking, underground only. It
    toggles a lever (`pull_lever` by id), opens a cage, springs a trap. Messages hint without
    naming it until a dwarf sees it.
  - **Proper kobolds (CRE-12).** New `Species::Kobold`: flees on sight, steals shiny things
    (gems, gold), sets a snare (a trap building that cripples), taunts ("A kobold jeers from the
    trees!"). A kobold camp site in the overworld can be raided (B18).
  - **Scavengers (CRE-06).** After a siege, carrion birds (`Species::Vulture`, harmless) gather
    at corpses outside; a troll (B16's Troll) may drag a body off; kobolds rob the dead.
  - **Thieving animals (CRE-26).** Magpies or monkeys by biome take small items. One that takes
    an artifact becomes a Figure ("Glint the magpie, thief of <artifact>").
  - **Night trolls (CRE-14).** New `Species::NightTroll`, unique per world (a Figure with a lair
    site). It comes at night, takes a spouse (captive with fate `Transforming`), and after a
    season the captive becomes its mate (a new figure). The lair is an expedition target where
    the gear of past victims is loot.
- **Steps:**
  1. Sneak goals refactor plus dogs seeing sneakers (no new behaviour beyond dogs) ·
     sneaks.rs, combat.rs (move `skulker_think` out) · existing thief tests pass, plus
     `a_dog_sniffs_out_a_skulker`.
  2. Baby snatchers with rescue and return · sneaks.rs, combat.rs `lay_siege_of` (turned
     captive spawn, one block) · done-when test
     `a_child_is_taken_at_night_and_the_fort_can_try_to_get_them_back`.
  3. Kobolds, gremlins and thieving animals · sneaks.rs, creature.rs species · test
     `a_gremlin_pulls_a_lever`.
  4. Scavengers and night trolls · sneaks.rs · test `a_night_troll_carries_off_a_spouse_to_its_lair`.
- **Overseer & screen:**
  - **Overseer:** new `Sense::Prowlers` (after a theft or snatching) leads to adopting dogs,
    locking doors at night (`DOOR_LOCKED` on outer doors from dusk), and keeping children in a
    burrow; Sentimental sends a rescue (B18).
  - **Screen:** a card for a snatching; a hint message for gremlins; the kobold taunts.
- **Measure:** batch snatchings and rescues per batch; deaths from new species small (< 3% of
  all deaths).
- **Risks / open questions:**
  - New species need `SpeciesInfo` and `Species::ALL` updates. Note that `ALL` has 18 entries
    and is used by `tame_from_cage`.
  - Night needs a day/night notion: use `tick % DAY > DAY * 3 / 4`.
- **Parallel work:**
  - **Writes:** new sneaks.rs (owned); creature.rs (species, `sneak`); combat.rs:
    `skulker_think` removed and `maybe_thief` delegating.
  - **Reads:** F0 Captive; B18 `Goal::Rescue`; B16 Troll and `lay_siege_of`'s turned captive.
  - **Hot spots:**
    - creature.rs `Species` enum and `info()`: append-only, but B16 also adds Troll, so
      coordinate in one shared "species" commit or order the waves.
    - combat.rs: removal of `skulker_think`.
  - **Agent size:** one agent, 2–3 sessions, in a wave after B18 and B16a.

### B22 Beasts of legend  (effort: 5 sessions; risk: high)
- **Design:**
  - **Powers (CRE-05).** New `Creature.power: Option<Power>`, with
    `Power::{Fire, Poison, Webs, Spit, Crystal, Iron, Statue}` (`breathes_fire` becomes
    `power == Some(Fire)`, migrated with serde). `make_cave_horror` derives the power from its
    words: ash → Fire, fungus or slime → Poison (a miasma-like cloud, reusing `miasma_cells`),
    spider → Webs (slows the tiles), serpent or lizard → Spit (ranged), crystal or iron → body
    armour multiplier, bone → Statue on death. Effects live in `apply_power` (combat.rs, called
    from `creature_attack`).
  - **The fort learns it.** `Game.bestiary: Vec<Lore { label, power, known: bool, seen_by:
    Option<u32> }>`. It becomes known when a dwarf within 8 tiles sees the power used. Look shows
    "breathes poison (seen by Urist)" or "???".
  - **Man-eaters (CRE-08).** A wild animal that kills a dwarf or hunter becomes a named Figure
    (`add_figure`), gets a lair site within 6 cells, keeps its wounds (`Body` serialized in the
    figure), and returns yearly via `maybe_megabeast`'s logic generalised to `beasts_near`.
  - **Disturbed lairs (CRE-09).** A failed `Goal::Beast` expedition sends `Party::Beast` to the
    fort, enraged, within days.
  - **Cyclic beasts (CRE-10).** `Figure.cycle: Option<u8>`; the beast returns every N years. A
    season before, a news item comes from the legends ("The old songs say Y wakes in the year
    Z").
  - **Monster hunters (CRE-07):** done in F07; temple lore waits for C10.
  - **Flying snatchers (CRE-13).** New `Mover::Flyer`, a path variant that ignores ground
    walkability outdoors (risky: pathing). Rocs grab a dwarf outdoors and drop them after N
    tiles from height (fall wounds).
  - **Dragon lairs and hoards (CRE-18).** `Figure.hoard: Vec<(ItemKind, Mat)>`. A dragon that
    survives takes items or livestock in the open over several visits, eggs hatch into figures
    yearly, and a Beast expedition's loot is the hoard.
  - **Hydras (CRE-21).** `Creature.heads: u8`: n attack rolls a turn, and heads are lost on a
    head-part wound, not death.
  - **Megabeasts take fallen sites (CRE-19).** In the B17 yearly pass, a beast that razed a
    site moves its lair there. The fort itself waits for P01.
  - **Generated megabeasts (CRE-25).** Worldgen `place_beasts` generates kinds ("frost dragon",
    "ash hydra") with a power. They breed (hatchling figures), and some parley: an Audience
    demanding tribute or livestock instead of attacking.
- **Steps:**
  1. Powers for cave horrors, plus lore learned · combat.rs (`make_cave_horror`,
     `apply_power`), creature.rs, new src/game/bestiary.rs · done-when test
     `a_cave_horrors_power_changes_how_it_fights_and_the_fort_learns_it_after_the_first_clash`.
  2. Man-eaters and disturbed lairs · bestiary.rs, expeditions.rs (a beaten Beast expedition
    calls `bestiary::provoked`) · test `a_bear_that_kills_a_hunter_gets_a_name_and_returns`.
  3. Cyclic beasts and generated megabeasts with powers · overworld.rs `place_beasts`,
    bestiary.rs · test `a_cyclic_beast_is_foretold_a_season_ahead`.
  4. Hydras and hoards · combat.rs `creature_attack` (heads loop), bestiary.rs · test
     `a_hydra_keeps_fighting_after_losing_a_head`.
  5. Flying snatchers (optional; needs the pathing decision) · path.rs, world.rs `Mover` · test
     `a_roc_drops_a_dwarf_from_the_sky`.
  6. Megabeasts take ruins, and bargaining beasts · overworld.rs, audience · test
     `a_dragon_demands_tribute_instead_of_attacking`.
- **Overseer & screen:**
  - **Overseer:** reads the bestiary. Known Fire means no wooden walls nearby and a squad with
    shields; Webs means ranged weapons; Poison keeps dwarves away. Cautious seals the caverns
    (door) when a known Poison beast is below. Each counter is a note.
  - **Screen:** a "Bestiary" tab in the status screen (in ui_world.rs), cards when a power is
    first seen, and a foretelling card.
- **Measure:** batch killers: cave horrors' share should not rise by more than 3 points. Look
  at a few `--story` runs for "learned that X breathes poison".
- **Risks / open questions:**
  - Flyers mean a new `Mover`, which touches path.rs (the hot path). Keep it optional or cut it
    to "grabs and throws on the same level".
  - Poison clouds reuse the miasma cells: watch performance.
- **Parallel work:**
  - **Writes:** new bestiary.rs (owned); combat.rs `make_cave_horror`, `creature_attack` (a
    power and heads hook: one call each); creature.rs (power, heads); overworld.rs
    `place_beasts`; expeditions.rs (one call).
  - **Reads:** F0 figures and Audience; F07 hunters; B17 yearly pass (for steps 3 and 6).
  - **Hot spots:**
    - combat.rs `creature_attack`: one call `self.apply_power(c, target)`.
    - creature.rs fields: append-only.
  - **Agent size:** too big for one session. Split into package B22a (steps 1–2, wave 2) and
    package B22b (steps 3–6, wave 4).

### B23 The restless dead  (effort: 5 sessions; risk: med-high)
- **Design:** a new module `src/game/restless.rs`, beside family.rs (ghosts) and necromancy.rs.
  - **Richer ghosts (MYT-01).**
    - New `Dwarf.ghost_kind: Option<GhostKind>` with `GhostKind::{Haunter { blames: u32 },
      Poltergeist, Violent, Guide}`, chosen at death:
      - drowned, with a lever pulled within 300 ticks that opened water → Haunter blaming the
        lever puller,
      - murdered or killed in a duel → Haunter blaming the killer,
      - hot temper → Violent,
      - otherwise Poltergeist or Guide.
    - Lever blame needs `Building.last_pulled_by: Option<(u32, u64)>`, set in the lever task.
      At a drowning death, find the floodgates or bridges linked from levers pulled recently
      upstream (a simple check: any lever pulled in the last 300 ticks whose linked gate is
      within 30 tiles of the body).
    - The Haunter visits the blamed dwarf each night: a stress thought and their own nightmare.
      Violent ghosts strike (bruise); poltergeists move items; Guides lead to the body (laying
      the ghost becomes easier).
  - **Horror at undead kin (MYT-02).** `raise_dead` keeps the corpse's name (labels exist).
    Spouses or children within 8 tiles get a heavy shock thought, and the ghost may walk at
    the same time.
  - **Dormant dead (MYT-17).** `Creature.dormant: bool` on undead placed in the vault or
    caverns at embark (5–12). They wake when a dwarf comes within 5 tiles or reveals their tile.
  - **Mummies and tomb curses (MYT-08).** `check_vault` may wake a mummy (new
    `Species::Mummy`) instead of a horror. It raises the coffined dead in tombs, and curses the
    looters (those who carried vault items): `Dwarf.cursed_until`, giving bad luck (doubled
    injury on falls and fights, and failed crafts).
  - **Undead from parts (MYT-16).** Scoped to evil land: corpses and carcasses outside rise as
    husks (exists) and now severed heads (a new item label from body part loss, if the body
    model has severing; otherwise cut).
  - **Evil with a cause (MYT-09).** The B17 necromancer figure and Tower site are an expedition
    target (`Goal::Beast(fig)` reused). Killing it lifts blood rain and husk spawning
    (weather.rs flags), and records "the fall of <region> into evil, and its cleansing".
  - **Spreading evil (MYT-20).** New `Weather::EvilFog`: a front from the evil side of the map
    that husks dwarves caught outside for N ticks. It fades and spreads one world cell a year
    in the B17 pass (`Cell.evil` on a neighbour).
  - **Secrets of death (MYT-19).** A slab of dark lore (`ItemKind::Slab` with a label) from the
    vault or bought from a goblin trade. A dwarf with high anxiety or fear of death reads it in
    a library or temple, and becomes a fort necromancer: raises the tomb dead at night; a crime
    for the sheriff (C07).
  - **Necromancer escalation (MYT-18).** Necromancer stages by years since known:
    1. raids,
    2. experiments (a "fused giant" of 2–3 bodies, `Species::Undead` with ×2 scale),
    3. a named intelligent lieutenant (a `general`-like leader whose death routs the undead),
    4. nightmares (a night raid).
- **Steps:**
  1. Richer ghosts and lever blame · restless.rs, family.rs `ghosts_tick` (delegating),
     buildings.rs (`last_pulled_by`), the lever job · done-when test
     `a_drowned_dwarfs_ghost_haunts_the_dwarf_who_pulled_the_lever`.
  2. Horror at undead kin, and dormant dead · restless.rs, necromancy.rs `raise_dead` (one
     block) · test `a_widow_sees_her_husband_walk`.
  3. Mummies and tomb curses · ruins.rs `check_vault`, restless.rs · test
     `looting_the_tomb_curses_the_looters`.
  4. Necromancer escalation and secrets of death · necromancy.rs, restless.rs · test
     `a_necromancer_raises_a_lieutenant_in_its_third_year`.
  5. Evil with a cause, and spreading evil (after B17) · weather.rs, overworld.rs · test
     `slaying_the_necromancer_ends_the_blood_rain`.
- **Overseer & screen:**
  - **Overseer:** treats a Haunter ghost as a trouble (`Lack::Rest` → a slab or burial
    priority); Sentimental buries faster; Pious builds a temple; Cautious walls off the vault
    after a mummy; Bold sends a squad to the tower.
  - **Screen:** a ghost card naming whom it blames; a curse mark on the unit sheet; a
    necromancer escalation card.
- **Measure:** batch deaths from the undead not up more than 2 points; ghost hauntings per fort
  as a report count.
- **Risks / open questions:**
  - Lever blame needs water attribution: keep it heuristic.
  - MYT-16 depends on body severing; cut it if severing doesn't exist.
- **Parallel work:**
  - **Writes:** new restless.rs (owned); family.rs `ghosts_tick` (delegate); necromancy.rs;
    ruins.rs `check_vault`; buildings.rs one field; weather.rs (step 5).
  - **Reads:** F0 (nothing essential); B17 for step 5.
  - **Hot spots:** dwarf.rs: 2 fields. combat.rs: none.
  - **Agent size:** one agent for steps 1–3 (wave 1–2), and a second package for steps 4–5
    (after B17).

### B10 Why you came  (effort: 2 sessions; risk: low)
- **Design:** a new module `src/game/scenario.rs`.
  - **Scenarios.** `pub enum Scenario { Frontier, MiningCompany, Citadel, PrisonColony, Inn,
    Shrine, RefugeeBand, ChildhoodFriends }` and `pub enum Backstory { None, FleeingWar(u32
    civ), Disgrace, ChasingRumour(u32 site) }`, stored in `Game.scenario` and `Game.backstory`
    (serde default Frontier/None).
  - **Choosing.** `c` on the embark world map cycles the scenario; the side panel shows its
    blurb. Autoplay picks by style: Warlord → Citadel, Delver → MiningCompany, Builder → Shrine,
    Steady → Frontier, Reckless → PrisonColony.
  - **Effects.** `Scenario::apply(&mut Game)` runs after the founders are made in `from_world`'s
    caller:
    - **Starting labours and skills:** mining company 4 miners and 3 picks; citadel 3
      soldiers with copper gear.
    - **Supplies.**
    - **Opening Standings and News:** fleeing war makes that civ −40 and sends an early army;
      disgrace makes home −30, with half the migrants in year 1; chasing a rumour adds a news
      item and a hidden site to `destinations` (F07 treasure trail).
    - **A charter goal** checked at year 2 (mining: 20 metal bars → home favour +20 and a
      migrant boost; citadel: survive two sieges; inn: 10 visitors).
    - **The migrant pool** (via B19 `requested_labors`) and **the caravan list lean** (B15).
- **Steps:**
  1. Scenario enum, choice UI and starting party and goods · scenario.rs, app.rs
     `key_world`/`start_embark` (pass the scenario), ui.rs world side panel (one block) ·
     done-when test
     `a_mining_company_starts_with_different_dwarves_goods_and_goals_than_a_citadel`.
  2. Backstories into Standing and News, the charter and its check, and the style pairing ·
     scenario.rs, overseer.rs `run_trial` (pick by style, one line) · test
     `fleeing_a_war_brings_that_enemy_early`.
- **Overseer & screen:**
  - **Overseer:** the charter goal biases ambitions (mining → Delve/Prosper +0.5 via
    `Sense::Charter`), and the note at embark says why we came.
  - **Screen:** the embark panel, the first chronicle line ("They came fleeing …"), and the
    status screen shows the charter's progress.
- **Measure:** batch per scenario (styles map to scenarios): survival not below M3 per style by
  more than 1 fort.
- **Risks / open questions:**
  - A prison colony needs C07 justice to mean much.
  - Decision for the user: which scenarios are in v0.6 (suggested: Frontier, MiningCompany,
    Citadel, RefugeeBand, ChildhoodFriends).
- **Parallel work:**
  - **Writes:** new scenario.rs; app.rs `key_world` (one key arm), `start_embark` (one
    parameter); ui.rs `draw_world` (one block).
  - **Reads:** F0 Standing/News; B19 `requested_labors`; B15 goods lean (optional).
  - **Hot spots:** game.rs: 2 fields and one `apply` call after the founders spawn. overseer.rs
    `run_trial`: one line.
  - **Agent size:** one agent, 2 sessions.

### B24 A wider market  (effort: 3 sessions; risk: med)
- **Design:**
  - **Agreements (ECO-21 = NOB-58).** `Standing.agreement: Option<Agreement { wants:
    Vec<(ItemKind, u8 premium_pct)>, brings: Vec<(ItemKind, u8 n)>, until_year: i32 }>`. It is
    negotiated at each caravan's arrival as an Audience (Offer::Trade). The liaison (broker)
    skill sets the premium. `trade()` values `ours` with `price(civ, kind)` (below) plus the
    premium.
  - **Messengers open trade (ECO-22).** B02's messenger to an unmet or far civ (up to 45
    cells) opens caravans: `Standing.trade_open = true`, and `caravan_season` includes it.
  - **Local prices (ECO-23).** `fn price(&self, civ, kind) -> f32` comes from:
    - the civ's home region (the biome of its capital, and of the caravan's `from` site):
      forests make wood cheap and metal dear, mountains the reverse, deserts dear food;
    - race tastes (elves pay no premium for wood and value gems);
    - saturation from `Standing.ledger.sold_last_year[kind]`: −5% per 5 sold, floor 50%.
    Merchant goods come from their region too: `caravan_goods(civ, from_site)` takes a biome
    table (jungle elves bring fruit and leather; badlands dwarves bring copper, no wood). The
    trade screen shows each item's price modifier.
  - **Tribute through caravans (ECO-19).** B02's tribute goes as goods into the home caravan's
    outbound load: `Party::Caravan` carrying a `cargo_value`. Bandit camps (B17) on the route
    can intercept it, and a lost tribute counts as unpaid unless the overseer re-pays. Tribute
    owed to us by a tributary (rare: after B17 conquests) comes as goods.
- **Steps:**
  1. Local prices and regional goods · new src/game/market.rs, trade.rs (`trade`'s value sum and
     `caravan_goods`) · done-when test
     `a_caravan_brings_what_its_home_region_produces_and_prices_move_with_what_the_fort_sold_last_year`.
  2. Agreements negotiated at the depot · market.rs, audience · test
     `an_export_agreement_pays_a_premium`.
  3. Messengers open far trade, tribute travels and can be raided · market.rs, worldturn.rs ·
     test `a_tribute_caravan_ambushed_by_bandits_counts_as_unpaid`.
- **Overseer & screen:**
  - **Overseer:** Greedy pushes agreements and sells into the premium (work orders for the
    wanted kinds); it sends messengers to open trade when Prosper is its ambition; it avoids
    selling a kind that is saturated.
  - **Screen:** the trade screen price column with arrows, the agreement terms in the
    Neighbours tab, and caravan cargo on the atlas.
- **Measure:** batch trade value per fort-year vs M3 (should rise for Greedy and Steady); the
  share of deals including agreement goods.
- **Risks / open questions:** values must stay integer in `trade()`. Round
  `value * price` and keep `offer_ok` integer.
- **Parallel work:**
  - **Writes:** new market.rs (owned); trade.rs `trade`, `caravan_goods` (B15 also edits
    `caravan_goods`, so B24 comes after B15).
  - **Reads:** F0 Standing/Audience; B02 messenger; B17 camps; B15 ledger.
  - **Hot spots:** trade.rs: 2 functions; overseer.rs `ov_trade`: the price-aware choice.
  - **Agent size:** one agent, 2–3 sessions, in wave 4.

---

## Interface contracts (build these in F0; dependents code against them)

All new `Game` fields are `#[serde(default)]`. All enums derive `Clone, Copy (where possible),
Debug, PartialEq, Serialize, Deserialize`.

```rust
// ---------- src/overworld.rs (additions) ----------
pub struct Site { /* existing … */
    #[serde(default, skip_serializing_if = "is_false")] pub player: bool,
    #[serde(default)] pub plundered: u8,          // B18
    #[serde(default)] pub hidden: bool,           // F07 treasure trails
    #[serde(default)] pub overlord: Option<u32>,  // B17
}
pub struct Figure { /* existing … */
    #[serde(default)] pub deeds: Vec<String>,     // B16
    #[serde(default)] pub site: Option<u32>,      // home site (B19)
    #[serde(default)] pub kin: Vec<u32>,          // B19, B17 WLD-32
    #[serde(default)] pub cycle: Option<u8>,      // B22
    #[serde(default)] pub power: Option<String>,  // B22 (name of crate::game Power)
    #[serde(default)] pub hoard: Vec<(crate::items::ItemKind, crate::material::Mat)>, // B22
}
pub struct Event { pub year: i32, pub text: String, #[serde(default)] pub kind: Option<NewsKind> }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartyKind { Army, Caravan, Envoy, Messenger, Hunter, Refugees, Squad, Beast, Bandits }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Party {
    pub id: u32,
    pub kind: PartyKind,
    pub civ: Option<u32>,
    pub from: u32,                 // site id
    pub to: u32,                   // site id (the fort's site for incoming)
    pub route: Vec<(i32, i32)>,    // world cells, from → to
    pub at: usize,                 // index into route
    pub size: u32,
    pub leader: Option<u32>,       // figure id
    pub depart: u64,               // game tick
    pub eta: u64,                  // game tick of arrival
    #[serde(default)] pub seen: bool,
    #[serde(default)] pub stealthy: bool,
    #[serde(default)] pub cargo_value: u32,
    #[serde(default)] pub note: String,  // "for the merchants slain in 253"
}
pub struct Road { pub a: u32, pub b: u32, pub cells: Vec<(i32, i32)> }
pub struct Overworld { /* existing … */
    #[serde(default)] pub parties: Vec<Party>,
    #[serde(default)] pub next_party: u32,
    #[serde(default)] pub roads: Vec<Road>,
}
impl Overworld {
    pub fn add_player_site(&mut self, name: &str, civ: Option<u32>, x: i32, y: i32, year: i32) -> u32;
    pub fn nearest_site_of(&self, civ: u32, x: i32, y: i32) -> Option<u32>;
    pub fn route(&self, from: u32, to: u32) -> Vec<(i32, i32)>;   // roads if built, else a line
    pub fn build_roads(&mut self);                                 // B01
    pub(crate) fn simulate_year(&mut self, rng: &mut Rng, year: i32); // B17 makes pub(crate)
    pub(crate) fn add_figure(&mut self, rng: &mut Rng, kind: &str, race: Option<Race>, civ: Option<u32>, born: i32) -> u32;
}
```

```rust
// ---------- src/game/worldturn.rs ----------
pub const CELLS_PER_DAY: u64 = 3;
pub const SCOUT_RANGE: i32 = 9;
impl Game {
    pub fn dispatch(&mut self, kind: PartyKind, civ: Option<u32>, from: u32, to: u32, size: u32, leader: Option<u32>) -> u32;
    pub(super) fn world_daily(&mut self);        // move, sight, arrive → self.party_arrives(p)
    pub(super) fn world_yearly(&mut self);       // refresh neighbors; B17 adds the history pass
    fn party_arrives(&mut self, p: Party);       // match p.kind → caravan_arrives / army_arrives / envoy_arrives / …
    pub fn home_site(&self) -> Option<u32>;
}
// Game fields: home_site: Option<u32>, world_rng: Rng (default from seed)
```

```rust
// ---------- src/game/relations.rs ----------
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wrong { HarmedMerchant, RobbedCaravan, AbusedEnvoy, RefusedDemand, RaidedSite, FelledTrees,
                 KilledLeader, BrokeSiege, HeldPrisoners, ExecutedPrisoner, SoldBanned, CheatedTrade }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grievance { pub wrong: Wrong, pub tick: u64, pub weight: i32, pub text: String }
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TradeLedger { pub visits: Vec<i32 /*merchant profit*/>, pub sold_last_year: BTreeMap<String, u32>, pub sold_this_year: BTreeMap<String, u32> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Standing {
    pub civ: u32,
    pub opinion: i32,                      // -100..=100
    #[serde(default)] pub met: bool,
    #[serde(default)] pub war: bool,       // the civ's own war on the fort
    #[serde(default)] pub grievances: Vec<Grievance>,
    #[serde(default)] pub truce_until: u64,
    #[serde(default)] pub muster_bonus: u8,
    #[serde(default)] pub ledger: TradeLedger,
    #[serde(default)] pub bans: Vec<crate::items::ItemKind>,
    #[serde(default)] pub agreement: Option<Agreement>,   // B24 (defined in market.rs, re-exported)
    #[serde(default)] pub trade_open: bool,
    #[serde(default)] pub wealth_at_last_tax: u32,
}
pub const WAR_LINE: i32 = -60;
impl Game {
    pub fn standing(&mut self, civ: u32) -> &mut Standing;   // creates with base opinion
    pub fn opinion(&self, civ: u32) -> i32;                  // base if absent
    pub fn wrong(&mut self, civ: u32, w: Wrong, weight: i32, text: String);
    pub fn favour(&mut self, civ: u32, n: i32, why: &str);
    pub fn grudge_line(&self, civ: u32) -> Option<String>;   // heaviest unexpired grievance
    pub fn fort_at_war_with(&self, civ: u32) -> bool;        // Standing.war
    pub(super) fn relations_yearly(&mut self);               // drift 20% to base, expire grievances > 10 yrs
}
// Game field: standings: Vec<Standing>, felled: u32 (reset yearly; last_felled: u32)
```

```rust
// ---------- src/game/audience.rs ----------
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ask { Tribute(u32), Artifact(u32), Prisoner(u32), Criminal(u32), Noble(u32), Soldiers(u32),
               Aid(u32 /*site*/), Lumber(u32), Hunt(u32 /*creature*/), Leave /*permission*/, Residence(u32), Livestock(u32) }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Offer { Peace, Trade, Barony, Ransom(u32), Withdraw, Agreement /*B24*/ }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Answer { Accept, Refuse, Abuse }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudienceKind { Envoy, Herald, TaxCollector, Hunter, Quester, Migrants, Prisoner, Beast, Merchants, Levy, Plea }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audience {
    pub id: u32, pub kind: AudienceKind, pub civ: Option<u32>,
    pub who: String,                 // "Ama Leafsong, envoy of the Green Accord"
    pub ask: Option<Ask>, pub offer: Option<Offer>,
    pub opened: u64, pub deadline: u64,
    pub answer: Option<Answer>,
    pub creature: Option<u32>,       // the envoy/herald on the map, if any
}
impl Game {
    pub fn open_audience(&mut self, a: Audience) -> u32;   // spotlight + pause_request (player) or overseer answers next oversee
    pub fn answer_audience(&mut self, id: u32, ans: Answer) -> String; // dispatches to the owner via kind
    pub(super) fn ov_answer(&self, o: &Overseer, a: &Audience) -> (Answer, String /*voiced reason*/);
    pub(super) fn audiences_tick(&mut self);                // expiry → Refuse; overseer answers
}
// Owners register consequences with one fn per kind: `fn on_answer_<kind>(&mut self, a: &Audience, ans: Answer)`
// in their own module (diplomacy.rs, combat.rs parley, visitors.rs hunter …); audience.rs only routes.
// Game field: audiences: Vec<Audience>; App: Mode::Audience { id: u32, sel: usize }
```

```rust
// ---------- src/game/news.rs ----------
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NewsKind { War{a:u32,b:u32}, Peace{a:u32,b:u32}, SiteFell{site:u32}, SiteFounded{site:u32}, RulerDied{civ:u32},
                    BeastSeen{fig:u32}, BeastSlain{fig:u32}, Muster{civ:u32}, Plot, Foretold{fig:u32}, Other }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source { Merchants, Migrant, Visitor, DeepRoads, Scouts, Legends }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct News { pub tick: u64, pub year: i32, pub text: String, pub kind: NewsKind, pub source: Source, pub true_: bool }
impl Game { pub fn hear(&mut self, n: News) -> bool /*false if already heard*/; pub fn news_since(&self, tick: u64) -> impl Iterator<Item=&News>; }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptiveFate { Held, Turned, Transforming, Dead }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Captive { pub dwarf: Dwarf, pub civ: Option<u32>, pub site: u32, pub since: u64, pub fate: CaptiveFate, pub taken_by: String }
impl Game { pub fn take_dwarf_off_map(&mut self, id: u32) -> Option<Dwarf>; pub fn make_captive(&mut self, id: u32, civ: Option<u32>, site: u32, by: &str) -> bool; pub fn free_captive(&mut self, id: u32, at: Pos) -> bool; }
// Game fields: news: Vec<News>, captives: Vec<Captive>, prisoners: Vec<u32> (creature ids, B02)

// ---------- src/dwarf.rs (fields, all #[serde(default)]) ----------
pub figure: Option<u32>, pub home_site: Option<u32>, pub came_for: Option<String>,
pub ghost_kind: Option<GhostKind> /*B23, enum in restless.rs re-exported*/, pub cursed_until: u64 /*B23*/
// ---------- src/creature.rs (fields, all #[serde(default)]) ----------
pub party: Option<u32>, pub parley_until: u64, pub carrying: Option<u32>, pub side: Option<u32>,
pub last_hit_by: Option<Attacker /*enum { Dwarf(u32), Trap, Creature(u32) }*/>,
pub sneak: Option<SneakGoal>, pub power: Option<Power>, pub heads: u8, pub dormant: bool
```

Report: `Trial.world: WorldNote` (`#[serde(default)]`), filled from `Game` in `run_trial`:
`warn_days: Vec<f32>`, `grievance_sieges: u32`, `envoys: Vec<(String, String)>`, `news: u32`,
`caravans: [u32; 3]` (came, skipped, robbed), `captives: [u32; 2]` (taken, rescued),
`known_migrants: u32`, `waves: u32`, `world_changes: u32`, `vermin_eaten: u32`, `hauntings: u32`.
Packages append their counters to this struct only (one line each).

## Hot-spot discipline (refactors to do before wave 1, by one agent, ~1 session)

1. **Register systems in `Game::step` through per-period hubs.** Add `fn daily_systems(&mut
   self)`, `fn season_systems(&mut self, season)` and `fn yearly_systems(&mut self)` in a new
   `src/game/systems.rs` holding today's call lists. `step` calls the three hubs. Each package
   adds **one line** to a hub, in a block marked `// M4: <item>`. Put the lines in alphabetical
   order so concurrent appends rarely touch the same line.
2. **Group the `Game` struct's new fields.** Add one `#[serde(default)] pub m4: M4State` field
   holding standings, audiences, news, captives, prisoners, felled, vermin, bestiary,
   camera_spots, map_notes, scenario and so on, defined in `src/game/m4state.rs`. Each package
   adds its fields to that struct in its own module's sub-struct (e.g. `pub pests:
   pests::PestState`), so `game.rs` changes once. Also update `from_world`'s field list once
   (`m4: Default::default()`).
3. **Split `combat.rs`** (2,044 lines, touched by B01, B15, B16, B20, B21, B22):
   - move `muster`, `maybe_siege`, `siege_strength`, `siege_leader`, `lay_siege_of`, `rout` and
     `army_arrives` to a new `src/game/siege.rs`;
   - move `skulker_think` and `maybe_thief` to `sneaks.rs`;
   - move `maybe_cave_horror`, `make_cave_horror`, `spawn_cave_horror`, `spawn_megabeast` and
     `dragon_hoard` to `bestiary.rs`;
   - move `wildlife` to `wildlife.rs`.
   Then B16 owns siege.rs, B21 sneaks.rs, B22 bestiary.rs, and B20 wildlife.rs.
4. **Split `creature_think` and `creature_tick`** into a per-kind dispatch,
   `match c.species.role() { Siege => …, Visitor => visitor_think, Merchant => merchant_think,
   Sneak => sneak_think, Beast => beast_think … }`, so B16, B21, F07 and B22 add arms in their
   own modules.
5. **Move `migrants()` out of game.rs** into `migrants.rs` (B19 step 1).
6. **Pre-cut ui.rs and app.rs.**
   - New `src/ui_world.rs` (atlas, Neighbours, News, Bestiary tabs) and `src/ui_minimap.rs`;
     `draw` and `status_lines` call into them.
   - `app.rs`: one `key_m4(&mut self, k) -> bool` hook at the top of `key_normal` that packages
     extend in a new `src/app_m4.rs`.
   - Add all new `Mode` variants (Atlas, Audience) in this refactor, so the enum changes once.
7. **Make `Species` additions one commit.** Troll, Kobold, Gremlin, Vulture, Magpie,
   NightTroll and Mummy are added with `SpeciesInfo`, `ALL` updated, and no behaviour, in the
   pre-wave refactor. Then B16, B21, B22 and B23 never conflict on creature.rs's enum.
8. **Sense, Lack and Trait enums.** Pre-add `Sense::{Threat, Rumour, Captive, Prowlers,
   Charter}` and `Lack::{Vermin, Rest}` with neutral weights, so ambitions.rs and troubles.rs
   only get per-item arms.

## Work packages and waves

Each package is one agent in its own worktree; a session is about one context-window of work.
"∥" means the packages can run at the same time (disjoint functions); "→" means in series.

**Wave 0 (serial, 1 agent, ~1–2 sessions):** P0 the hot-spot refactor (all 8 items above, no
behaviour change; all tests green; the batch matches M3 exactly with the same seed, because no
RNG calls moved), then **F0** (packages F0a: site, party, worldturn; F0b: relations, audience,
news, captives). F0a ∥ F0b if P0 has landed the `M4State` struct.

**Wave 1 (after F0), all in parallel:**

| Package | Items/steps | Owns | Sessions |
|---|---|---|---|
| W1-A | B01 steps 1–2 (caravans and armies as parties, scouts) | siege.rs `muster`/`army_arrives`, trade.rs `caravan_season` split, worldturn.rs arrival arms | 2 |
| W1-B | W25 all | ui_minimap.rs, app_m4.rs keys, status Places tab | 2–3 |
| W1-C | B20 steps 1–4 | pests.rs, wildlife.rs, farming.rs fishing | 3 |
| W1-D | B23 steps 1–3 | restless.rs, family.rs `ghosts_tick`, ruins.rs `check_vault`, buildings.rs field | 2–3 |
| W1-E | B22 steps 1–2 (powers, lore, man-eaters) | bestiary.rs, `creature_attack` hook | 2 |
| W1-F | B19 steps 1–3 | migrants.rs | 2 |

Serialize: W1-A and W1-E both touch combat.rs only through their moved modules (siege.rs vs
bestiary.rs), so they're fine. W1-E's disturbed-lairs hook in expeditions.rs is one line;
expeditions.rs isn't owned by anyone in wave 1.

**Wave 2 (after W1-A):**

| Package | Items/steps | Owns | Sessions |
|---|---|---|---|
| W2-A | B01 steps 3–4 (roads, atlas) | overworld.rs `build_roads`/`route`, ui_world.rs atlas | 2 |
| W2-B | B02a (envoys, abuse, tax) | diplomacy.rs, visitors.rs ENVOY branch, trade.rs `caravan_arrives` collector | 2–3 |
| W2-C | B18 all | expeditions.rs, ambitions.rs `ov_venture` | 3 |
| W2-D | B16a (leader, grudges, beasts) | siege.rs `siege_leader`, `muster` weights, `lay_siege_of` | 2 |

Conflicts: W2-B and W2-D both touch `muster` (W2-B adds a truce filter line, W2-D the
weights), so let **W2-B land first**, or give W2-D ownership of `muster` and have W2-B
contribute only `Standing.truce_until`, which W2-D reads. Recommended: W2-D owns `muster`. W2-A
and W2-C both draw on the atlas, through separate overlay providers
(`ui_world::overlays(game) -> Vec<Overlay>` built in W2-A; W2-C adds the squad route through
`Party` data only, so no UI code).

**Wave 3:**

| Package | Items/steps | Owns | Sessions |
|---|---|---|---|
| W3-A | B17a (yearly pass, wars, news carriers) | overworld.rs `simulate_year`/`battle`, worldturn.rs `world_yearly`, news.rs | 3 |
| W3-B | B15 all | trade.rs (except `caravan_arrives`'s news call), needs.rs fallback, overseer.rs `ov_trade` | 3 |
| W3-C | B16b (parley, carry, sides) | siege.rs `creature` siege role, audience owner for Herald | 2–3 |
| W3-D | B02b (messengers, prisoners, barony/levy/pleas) | diplomacy.rs, traps.rs | 2–3 |
| W3-E | F07 steps 1–3 | visitors.rs | 2 |

Conflicts: W3-A's merchant-news call sits in `caravan_arrives`, while W3-B edits trade.rs
elsewhere, so this is a small merge (fine). W3-D and W3-A both relate to the levy (it needs
B17 wars): put the levy last in W3-D, behind a `fort_civ_at_war()` check that reads
`overworld` and works either way.

**Wave 4:**

| Package | Items/steps | Owns | Sessions |
|---|---|---|---|
| W4-A | B17b (towns rise, camps, alliances, the necromancer on the map, linked figures) | overworld.rs | 3 |
| W4-B | B24 all | market.rs, trade.rs `trade`/`caravan_goods` | 2–3 |
| W4-C | B21 all | sneaks.rs | 3 |
| W4-D | F07 step 4 (questers, recovery, trails) | visitors.rs, expeditions.rs `Goal::Recover` | 1–2 |
| W4-E | B19 step 4 (refugees, baggage) | migrants.rs | 1 |

**Wave 5:**

| Package | Items/steps | Owns | Sessions |
|---|---|---|---|
| W5-A | B22b (cycles, hydras, hoards, flyers, bargaining) | bestiary.rs, path.rs if flyers | 3 |
| W5-B | B23 steps 4–5 (escalation, evil with a cause, fog) | necromancy.rs, weather.rs | 2 |
| W5-C | B20 step 5 (livestock raids, needs B21 sneaks) | pests.rs | 1 |
| W5-D | B10 all | scenario.rs | 2 |

**Coordinator batches:** one `--autoplay --runs 25 --years 3 --seed 5000` after each wave merges
(waves 0, 1, 2, 3, 4, 5), compared with the previous wave's JSON. Wave 0 must match M3's batch
exactly, since it is a pure refactor. Waves 2–3 are the balance-sensitive ones (scout warnings,
parley, caravans).

## Verification per package

| Package | Tests the agent adds (all in the owning module's `#[cfg(test)] mod tests`) | Batch after merge |
|---|---|---|
| P0 | none new; all existing green; `one_year_timing` unchanged | yes (must equal M3) |
| F0a | `the_fort_is_a_site_on_the_world_map`, `a_party_walks_from_its_site_to_the_fort_in_days` | no |
| F0b | `wrongs_sour_a_civ_and_fade_slowly`, `the_overseer_answers_by_its_traits`, `news_is_heard_once_and_captives_survive_a_save` | no |
| W1-A | `a_caravan_travels_the_world_map_before_it_arrives`, `a_siege_is_announced_days_ahead_and_seen_approaching`; S03 tests still pass | wave 1 |
| W1-B | `clicking_the_minimap_moves_the_view`, `saved_camera_spots_jump_back`, `a_typed_world_name_gives_the_same_world`, `map_notes_persist_and_show`, `stock_search_filters` | no |
| W1-C | `a_hoarded_food_store_draws_vermin_and_a_fort_with_cats_loses_less`, `a_hungry_dog_kills_a_cat_and_the_owners_fall_out`, `overfishing_empties_a_bank_and_it_recovers`, `felling_a_savage_forest_brings_enraged_beasts` | wave 1 |
| W1-D | `a_drowned_dwarfs_ghost_haunts_the_dwarf_who_pulled_the_lever`, `a_widow_sees_her_husband_walk`, `looting_the_tomb_curses_the_looters` | wave 1 |
| W1-E | `a_cave_horrors_power_changes_how_it_fights_and_the_fort_learns_it_after_the_first_clash`, `a_bear_that_kills_a_hunter_gets_a_name_and_returns` | wave 1 |
| W1-F | `a_deadly_year_thins_the_next_wave_and_says_why`, `a_migrant_arrives_with_a_reason_and_someone_already_knows_them` | wave 1 |
| W2-A | `roads_join_sites_and_avoid_the_sea`, atlas dump test | no |
| W2-B | `paying_tribute_delays_a_siege_and_refusing_brings_it_sooner`, `killing_an_envoy_brings_war`, `refusing_the_tax_collector_costs_home_favour` | wave 2 |
| W2-C | `an_expedition_walks_its_route_and_returns_after_its_length`, `a_beaten_expedition_leaves_a_captive_and_a_later_one_brings_them_home`, `raiding_a_site_twice_brings_less_loot_and_a_counter_raid`, `a_returning_soldier_grieves_a_spouse_who_died_while_away` | wave 2 |
| W2-D | `the_warning_card_names_the_leaders_deeds`, `a_siege_leader_names_the_wrong_and_their_army_was_on_the_map`, `a_general_brings_a_named_war_beast` | wave 2 |
| W3-A | `after_three_years_the_legends_show_wars_and_deaths_elsewhere_that_the_fort_heard_about`, `a_civ_soured_enough_declares_war_on_the_fort`, `merchants_bring_news_and_some_of_it_is_false` | wave 3 |
| W3-B | `a_hungry_fort_is_brought_food`, `a_soured_merchant_wants_more_and_says_why`, `robbing_a_caravan_sours_the_civ_and_its_next_caravan_does_not_come`, `unprofitable_visits_thin_the_caravans`, `elves_refuse_wood_from_a_fort_that_fells_forests` | wave 3 |
| W3-C | `paying_at_the_parley_sends_the_army_home`, `a_downed_dwarf_is_carried_off_and_becomes_a_captive`, `two_enemy_armies_fight_each_other_at_the_gate` | wave 3 |
| W3-D | `a_messenger_makes_peace_with_a_friendly_civ`, `a_caged_goblin_is_ransomed_or_executed_by_traits`, `a_levied_squad_comes_home_with_veterans_or_not` | wave 3 |
| W3-E | `a_hunter_arrives_when_a_cave_horror_lives_and_goes_after_it`, `a_teller_tells_a_tale_from_the_legends_and_it_becomes_news`, `a_slain_beast_raises_fame_with_present_visitors_civ` | wave 3 |
| W4-A | `a_razed_town_is_resettled_in_time`, `a_bandit_camp_appears_in_a_ruin_and_raids_the_road`, `an_alliance_marches_against_the_necromancer`, `a_migrants_brother_back_home_marries_and_dies_in_the_legends` | wave 4 |
| W4-B | `a_caravan_brings_what_its_home_region_produces_and_prices_move_with_what_the_fort_sold_last_year`, `an_export_agreement_pays_a_premium`, `a_tribute_caravan_ambushed_by_bandits_counts_as_unpaid` | wave 4 |
| W4-C | `a_dog_sniffs_out_a_skulker`, `a_child_is_taken_at_night_and_the_fort_can_try_to_get_them_back`, `a_gremlin_pulls_a_lever`, `a_night_troll_carries_off_a_spouse_to_its_lair` | wave 4 |
| W4-D | `a_quester_steals_an_artifact_and_it_can_be_recovered` | no |
| W4-E | `refugees_arrive_with_pursuers_behind_them` | wave 4 |
| W5-A | `a_cyclic_beast_is_foretold_a_season_ahead`, `a_hydra_keeps_fighting_after_losing_a_head`, `a_dragon_demands_tribute_instead_of_attacking` (+ `a_roc_drops_a_dwarf_from_the_sky` if flyers) | wave 5 |
| W5-B | `a_necromancer_raises_a_lieutenant_in_its_third_year`, `slaying_the_necromancer_ends_the_blood_rain` | wave 5 |
| W5-C | `an_unguarded_pasture_loses_a_goat_at_night` | wave 5 |
| W5-D | `a_mining_company_starts_with_different_dwarves_goods_and_goals_than_a_citadel`, `fleeing_a_war_brings_that_enemy_early` | wave 5 (final M4 batch) |

Every package must leave `cargo test --release` printing `test result: ok`. Anything touching
the world tick, sieges or pathing also runs `one_year_timing` (target: within +5% of the wave
before).

## Estimated effort

| Item | Sessions | | Item | Sessions |
|---|---|---|---|---|
| P0 refactor | 1–2 | | B17 | 5 |
| F0 | 3 | | B18 | 3 |
| B01 | 3 | | B19 | 3 |
| B02 | 5 | | B20 | 4 |
| F07 | 3 | | B21 | 3 |
| W25 | 3 | | B22 | 5 |
| B15 | 3 | | B23 | 5 |
| B16 | 4 | | B10 | 2 |
| | | | B24 | 3 |

**Total: about 58 sessions.** With the waves above and 4–6 agents at once, that is roughly 6
waves, or about 12–15 sessions of wall-clock time plus the coordinator's batches.

## Cross-milestone notes

- **One world bus.** `Party` plus `world_daily` is how anything reaches the fort (armies,
  caravans, envoys, hunters, refugees, beasts) and how the fort reaches out (squads,
  messengers). M5 B03 "Armies abroad", M8 P03 roads and outposts, and P05 caravans on the map
  all build on it. P05's ECO-32 is in effect done here.
- **One opinion store.** `Standing` (per civ: opinion, grievances, truce, ledger, agreement) is
  shared by B02, B15, B16, B17, B18, B19, B24 and F07 (fame). C06 nobles' favour (M3) should
  use the same shape for the home civ's nobles, so "noble displeasure" and "home opinion" don't
  diverge.
- **One decision type.** `Audience` plus `ov_answer` covers envoys, heralds and parleys, tax,
  hunters, questers, migrants turned away, prisoners, levies and beast bargains. M3's C06
  mandates and C10 petitions should be ported onto it, so the overseer's traits decide
  everything in one table and the player sees one popup style.
- **News is the inbound twin of `word.rs`.** Keep `word.rs` for outbound reputation and add
  `news.rs` for inbound. B22's foretelling, B19's reasons, F07's tellers and B17's digest all
  read `News`. M2's O09 "legends worth telling" should draw on `Event.kind`.
- **Captives and figure links** (`Dwarf.figure`, `home_site`, `Captive`) serve M1 S03
  (captives), B16, B18, B21, P01 (survivors return) and C05 family sagas. Do them once in F0.
- **Refactors to do first:**
  - systems hubs in `step`;
  - an `M4State` sub-struct;
  - split combat.rs into siege, sneaks, bestiary and wildlife;
  - `creature_think` dispatch by role;
  - migrants out of game.rs;
  - ui_world.rs, ui_minimap.rs and app_m4.rs;
  - one commit for new species;
  - pre-added Sense and Lack variants.
  Together they cut the conflicting hot spots to a handful of single-line appends.
- **Determinism:** all world-side randomness uses `Game.world_rng`, so batch comparisons
  between waves stay meaningful and P0 can be verified as a no-op.
