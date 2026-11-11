## 1. Find
- [x] 1.1 Routine tables for TTerrainMesh, RF_Terrain, TTerrainManager and RF_kdTrees against the workspace
- [x] 1.2 REA: the block activation's triangle list, its callers and the hole mask layout

## 2. Port
- [x] 2.1 The holed block triangle list, wired into the tile build (draw and collider)

## 3. Check
- [x] 3.1 GPU grid against r191: exanima01 pit view with and without the wireframe, FPS
- [x] 3.2 Lockstep traces (exanima01, arena duel) and the determinism grid
- [x] 3.3 fmt, clippy -D warnings, workspace tests, play build, openspec --strict
