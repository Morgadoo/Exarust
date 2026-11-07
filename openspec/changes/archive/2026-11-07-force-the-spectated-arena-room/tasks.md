## 1. Find
- [x] 1.1 REA: the walk's forced branch, its far door-test point and the reach compare against the assembly
- [x] 1.2 Disassembly: every writer of the stored room (XA_Arena match starts and tick, the save unit's reset) and the spawn-spot table copy

## 2. Port
- [x] 2.1 `RoomTracker::update_forced` and `ForcedRoom` in the walk system
- [x] 2.2 The spectate session's forced point, fed while the match scene exists
- [x] 2.3 Tests; crate READMEs

## 3. Check
- [x] 3.1 GPU grid: a spectated match (`--hub-employee-kit Ash:282 --hub-enter 1`) logs the forced walk in the arena and its end in the hub; arena and exanima01 fixed-frames traces identical to main
- [x] 3.2 fmt, clippy -D warnings, tests, play build, openspec --strict
