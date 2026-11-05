# Tasks

- [x] Trace the coin control's procedure and draw and the sprite-sheet loader in REA, against the disassembly.
- [x] Trace the preview-bars switch, its draw callback and its callers.
- [x] Trace the unit's control-class registration and the record table's references (unreachable).
- [x] `exa_items::item_gui`: the coin baseline, cell and draw plan; tests.
- [x] `exa_game::item_card`: the coin sheet at the coin side, icons and counts on the live card; the rating dots' ink; tests.
- [x] `exa_ui::hud`, `exa_ui::game_gui`: the bars' painter without the red floor for the preview.
- [x] `exa_game::loadout_kit`: the preview bars on bind, drawn over the preview; tests.
- [x] Crate READMEs (docs/formats and docs/PLAN.md unchanged: no format change; PLAN.md is the main session's).
- [x] GPU check in the one-window grid: the card's coin icon and the loadout preview's bars (the real installation's sheets; skipped without `EXANIMA_DIR`).
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker (`make check` equivalent).
