# DwarfClone

A Dwarf Fortress-style colony simulator for the terminal, written in Rust with ratatui.
Fortress mode only (for now): you don't control dwarves directly — you designate work,
lay out zones and queue orders, and the dwarves get on with it according to their labors,
needs and moods.

## Run

```
cargo run --release            # random world
cargo run --release -- 12345   # specific seed
```

Use Windows Terminal (or any truecolor + Unicode terminal) at 130x40 or larger.

## What's in v0.1

- **World**: 96×64×32 generated region — hills with natural ramps, lakes, forests, soil
  over sedimentary / igneous rock layers, ore veins, gems, and a hidden cavern layer with
  pools and tower mushrooms. Unexplored rock stays hidden until dug into.
- **Designations**: mine, channel, up / down / up-down stairs, ramps, chop trees.
  Boxes can span z-levels (dig a staircase in one go).
- **Dwarves**: labors and skills (8 labors, 11 skill ranks), thirst / hunger / sleep,
  thoughts and stress, tantrums, death by starvation, thirst or drowning.
- **Economy**: stockpiles with categories, hauling, farm plots (stoutcap mushrooms),
  Still (brew ale), Carpenter's and Mason's workshops with repeatable work orders,
  furniture placement (beds are slept in). Dwarves dislike drinking plain water.
- **Simulation**: 3D A* pathfinding over stairs and ramps, cellular water that falls and
  levels out (channel next to a lake and watch), seasons, migrant waves.
- **Save / load**: F5 / F9 (`dwarfclone_save.json`).

Press `?` in game for keys.

## Tests

```
cargo test --release
cargo test --release one_year_timing -- --ignored --nocapture   # simulate a full year
```

## Layout

| File | Role |
| --- | --- |
| `world.rs` | tiles, movement rules, reveal, world generation |
| `game.rs` | simulation: jobs, needs, fluids, farms, migrants, player commands |
| `dwarf.rs` | dwarves, labors, skills, tasks, names |
| `buildings.rs`, `items.rs`, `material.rs` | data definitions |
| `path.rs` | A* and nearest-goal Dijkstra search |
| `app.rs`, `ui.rs` | input modes and rendering |
