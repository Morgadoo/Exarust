## 1. Find
- [x] 1.1 FLX_Terrain_Texrender2 and TStaticTerrain routine tables against the workspace
- [x] 1.2 REA: every reference to the blur kernel and its scratch planes; the one-block shadow draw's test and the cube faces' dirty rule

## 2. Port
- [x] 2.1 Cube faces leave out the terrain blocks the main view did not draw, faces kept cached as rendered
- [x] 2.2 Document the blur as unreachable in 0.9.5

## 3. Check
- [x] 3.1 GPU: arenao1/arenao3/arenao4 (fixed seed) and exanima08 before/after and FPS; the composite draw against the per-pixel draw; exanima01 and the arena duel traces identical to round 185
- [x] 3.2 fmt, clippy -D warnings, tests, play build, openspec --strict
