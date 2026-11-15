## 1. Read the original

- [x] 1.1 The save unit's open rows, the power classes' save and load slots and their callers in the assembly
- [x] 1.2 The placement writer's power walk, the record build's restore walk, the Mind and Force restore rows, Loyalty's and Command Dead's ready ticks, the spawn's activation walk
- [x] 1.3 The GUI reset and its callees, the journey's save, the slot delete

## 2. Wire

- [x] 2.1 Running powers in the placement format (read, write, level export)
- [x] 2.2 The live save of the player's running powers (the level section writes none, as the original's)
- [x] 2.3 The load: the player's and the placed characters' powers rebuilt, reserved and activated
- [x] 2.4 The loaded Loyalty and Command Dead relink by character id
- [x] 2.5 The GUI reset on every level change (death fade, screens, crystal sources, icon cache)
- [x] 2.6 The journey's save and checkpoint; the delete's log line

## 3. Tests and checks

- [x] 3.1 Format tests: the placement's running powers, the player placement round trip, the level section writing none
- [x] 3.2 Live-system tests: save then load with running powers, a retail-format record with Loyalty, the level change reset, the journey save
- [x] 3.3 No corpus check applies: no shipped level or retail save at hand holds a running power (EXANIMA_DIR checks unchanged)
- [x] 3.4 docs/formats/SAVES.md and RFC.md, the crate READMEs; `make check` (fmt, clippy -D warnings, tests)
