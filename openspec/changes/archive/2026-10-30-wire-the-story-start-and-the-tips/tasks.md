# Tasks

- [x] Trace the story start's record writes, spawn request, torch routine and its tip calls (REA)
- [x] Trace the tip popup's callers and table, and the menu's MANUAL rows (REA)
- [x] Hold a batch's steps after its level load until the player stands in the loaded level
- [x] Apply `StoryPlayer` to the spawn record, `StartRecord` to the spawned player, `StoryLight` (torch only without a carried one) and `LevelTime`
- [x] Show `Step::Tip` on the banner; hide it on the prelude's close and a load
- [x] Open, drive and paint the manual from the MANUAL rows; the menu yields to it
- [x] `--open-creator begin:N` for unattended story starts
- [x] Tests with the original's numbers (manual pack synthetic; no game data)
- [x] GPU check in `exanima01`: new story start, knock-out tip, F1 and MANUAL
- [x] Remote fmt, clippy -D warnings and tests on exa_ui, exa_game, exarust
