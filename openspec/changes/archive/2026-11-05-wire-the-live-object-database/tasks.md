# Tasks

- [x] Trace the tiers' set-up, the lazy base loader, the tier load and migration pass, the level reader's local-database chunk, the save export, import, pre-save sync, door transition, checkpoint, arena save, arena carry and new game in REA.
- [x] `exa_formats::objdb`: tier swaps, world export/import, carried-items and record promotion, container children write; tests.
- [x] `exa_formats::rfc`: the level's records as a tier store; corpus test over every level with a local database (skipped without `EXANIMA_DIR`).
- [x] `exa_assets::live_objects`: the live resource and its write log; tests.
- [x] Level load fills the local tier; `LevelLocalRecords` removed.
- [x] Use cursor kits on the live database; the copy rewrites the pack entry; `DollItems` and the catalogue read the tagged records from it; container popups write into it; kit passes copy through it.
- [x] Saves: world sync, `#worldobjdb` with the world library, load import and migration, level-change sync, new-story clear; round-trip test.
- [x] `--kit-use` debug script; docs/DEBUGGING.md and crate READMEs.
- [x] docs/formats/OTHER.md: The live tiers.
- [x] GPU check in `play exanima01`: clean and repair, save, reload, card.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker.
