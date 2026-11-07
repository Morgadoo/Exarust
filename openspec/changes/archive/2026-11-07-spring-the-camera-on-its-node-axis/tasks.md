## 1. Find
- [x] 1.1 Routine table of the camera controller against the workspace (live, model, missing)
- [x] 1.2 REA: the tick's heading and zoom sections, the frame placement, the placement reset, the wall routine and its walk, read in the assembly
- [x] 1.3 Corpus: the shipped meshes with both probe bits (`shipped_wall_probe_obstacles`)

## 2. Port
- [x] 2.1 The backward vector springs to the node axis; the shown yaw is its heading, the target wraps with it
- [x] 2.2 The zoom spring drives the field of view; the stored shown zoom follows past 0.01
- [x] 2.3 The wall tilt eases at the zoom-spring rate; the wall probe is answered by the drawn obstacles
- [x] 2.4 Tests; docs/PLAYER.md and the crate READMEs

## 3. Check
- [x] 3.1 GPU grid against main: the camera on an outdoor level near an obstacle, the zoom, exanima01 and the arena duel traces
- [x] 3.2 fmt, clippy -D warnings, tests, play build, openspec --strict
