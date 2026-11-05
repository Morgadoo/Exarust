# Tasks

- [x] Trace the save routine, the exporter's local-database chunk, the full load, scene load and transition hooks, the archive pass, the arena save's promotion, the followers' promotion, the fighter-kit preparation and the database loaders in REA; check the hand-pair table and the archive pass's first call in the assembly.
- [x] `exa_formats::rdb`: the table's count; corpus check of every `.rdb` and level database's last slot; synthetic tests updated.
- [x] `exa_assets::live_objects`: local tier migration on every level load.
- [x] `exa_world`: `LevelSource`, `PlacedIndex`, `SavedLevelSection`, `LevelLoad::begin_from`; the viewer passes the saved section.
- [x] `exa_game::level_save`: placed-object rewrite and section export; tests (synthetic and `exanima01` round trip, skipped without `EXANIMA_DIR`).
- [x] Saves write the section; the story load reads it; container fill gated and written into the local tier.
- [x] Arena promotion of the stash, arsenal, staff and scene ids; followers' promotion; tests.
- [x] Practice kit carry split around the load; flow test.
- [x] `--hub-stash`; `--gui-drop` reuses a held entry; docs/DEBUGGING.md, docs/formats/SAVES.md and OTHER.md, crate READMEs.
- [x] GPU checks: `exanima01` save and load (the level read from the save's section, the dropped and repaired trousers kept), `arenahub` save and load (stash and arsenal kept).
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker.
