## 1. Read the original

- [x] 1.1 The scene clear's shape reset and the dirty-marking slot's callers
- [x] 1.2 The injection's draw order, blending and the emissive programs' alpha
- [x] 1.3 The splat's frame, the terrain fill and the dynamic update's box
- [x] 1.4 The light kinds behind the program variant

## 2. Port

- [x] 2.1 The shape list's clear and order
- [x] 2.2 The per-pass blends, over dark voxels too, and the stored alpha
- [x] 2.3 The centimetre splat, the tile-grid terrain columns, the node-bounds box

## 3. Tests and checks

- [x] 3.1 Live-system tests: a teardown forgets the shapes, the shapes inject in list order, a level build reads the tile grid, a moving mesh is boxed by its node's bounds
- [x] 3.2 fmt, clippy -D warnings, the workspace tests, openspec validate, the play build
- [x] 3.3 Worker GPU before/after pictures and FPS on exanima01 and arenao3
