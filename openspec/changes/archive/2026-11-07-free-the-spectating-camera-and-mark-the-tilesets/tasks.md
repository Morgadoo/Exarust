## 1. Find
- [x] 1.1 REA: the tick's free branch and the unfollowed frame placement, read in the assembly
- [x] 1.2 REA: the wall routine's start and follow test; the walk's mark word and its writer
- [x] 1.3 REA: the tileset loader's mark; corpus of the shipped sets

## 2. Port
- [x] 2.1 `FreeFocus` and `free_pull`; the chase camera's free branch while spectating
- [x] 2.2 The probe only when following, from the hip, skipping cut-away-marked meshes
- [x] 2.3 `mark_tileset`, `tileset_model`; grid set pieces as obstacles
- [x] 2.4 Tests; docs/PLAYER.md and the crate READMEs

## 3. Check
- [x] 3.1 GPU grid against main: exanima01 and the arena duel traces, a dungeon wall behind the player, the spectating camera panned
- [x] 3.2 fmt, clippy -D warnings, tests, play build, openspec --strict
