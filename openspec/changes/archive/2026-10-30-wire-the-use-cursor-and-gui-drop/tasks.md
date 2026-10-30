# Tasks

- [x] Trace the use hook's caller, the cursor callback's end event, the dispatcher gate, the GUI drop's claim and task, the `GameEvent` registration and unlock write, the light toggle and the attach mark (REA)
- [x] `panel_use_class`, `UseCursor`; the live use cursor (`use_cursor`) with the door unlock and `--use-item`
- [x] The GUI drop into the world (`gui_drop`, `hand::lift_gui_drops`) with `--gui-drop`
- [x] The flow's `GameEvent` consumer and the background unlock write
- [x] The light toggle reads the held part's mark (`HeldMark`); the doll take-in sets it; `--doll-drop`
- [x] Unit and app tests; GPU checks of the use cursor, the drop and the torch toggle in exanima01 (real installation)
- [x] Docs: docs/PLAYER.md, crates/exa_game/README.md
- [x] fmt, clippy -D warnings and tests on exa_game, exa_items and exarust
