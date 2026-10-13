# DwarfClone

A Dwarf Fortress-style colony sim in Rust with ratatui. See README.md for features and layout.

## Working from the wishlist

- `WISHLIST.md` is the roadmap. Unless asked for something else, take the next unticked
  step from "Up next" in the suggested order.
- In the same commit as the feature, tick the step (`[x]` plus the commit hash, or "this
  commit"), and when an item is finished move it to "Done" with a one-line summary.
- New ideas from play go in "Inbox" with a reason.
- Each item's "Done when" describes the automated test to add.

## Conventions

- Original code and names only. Don't copy DF code, art or raw data. Keep invented names
  (stoutcap, our month names).
- Use DF's key conventions where they exist (`i` zones, `k` look, `z` status…).
- Before committing, `cargo test --release` must print `test result: ok`. Don't commit
  on a grep that only checks the command ran.
- A simulated year takes about 5 seconds (`cargo test --release one_year_timing -- --ignored --nocapture`).
  Re-check it after changing job search or pathing.
- For multi-line source patches, use Python scripts in the scratchpad. Shell heredocs
  break on apostrophes in Rust code.
- To look at a player's save: copy it out of `saves/` first, then
  `FORT=<copy> cargo test --release inspect_fort -- --ignored --nocapture` prints buildings,
  zones, dwarves, the most frequent announcements and a month of simulation timing.
