## 1. Terrain

- [x] 1.1 Packed per-layer height, gloss and mask textures with mips
- [x] 1.2 Gloss from the gloss textures; albedo × 0.75
- [x] 1.3 Parallax from the bump slot at the original's scale

## 2. GI

- [x] 2.1 Terrain voxelised below its surface
- [x] 2.2 Lights near the block injected, nearest first

## 3. Viewer

- [x] 3.1 `--torch`, `EXA_GI_DEBUG`, sector floors in the GI dump

## 4. Checks

- [x] 4.1 exanima01 log room at the game's camera: the floor's stones, finish and relief match; bounce light reaches the shadows
