# Tasks

- [x] Read the container, random-table and model sections, with synthetic tests and the shipped condition word.
- [x] Keep possessions as the record does: swap, held marks, give, has item, hands, loose items, cores, the AI's arms; read the record's hand flags.
- [x] Put possessions on placed characters and the player; build the catalogue; Switch Equipped swaps the player's sets.
- [x] Model the inventory window, the arsenal, the icon cache and the item GUI controls, with tests pinned to the original's numbers.
- [x] Model item copies: condition with variants, price, desirability, protection, rolls, palettes, the apparel roll and the refresh.
- [x] Model grades and the random-table fill.
- [x] Tint, clamp and lay out item panel entries.
- [x] Check the new sections against `objdb.rdb` (24 tables, 137 containers); the real-data check is the corpus statistic in `docs/formats/OTHER.md`, skipped without `EXANIMA_DIR`.
- [x] Update `docs/formats/OTHER.md`, `docs/PLAYER.md`, `docs/UI.md` and the `exa_items` README. `docs/PLAN.md` is the integrator's.
- [x] `make check` equivalent on the remote worker: tests and clippy `-D warnings` for the touched crates.
