## 1. Cube pass

- [x] 1.1 Pool of cube faces, slots taken by lights that need faces
- [x] 1.2 Needed and dirty faces; jobs extracted to the render world
- [x] 1.3 Caster pipeline: linear depth over R·k_c, back-face culling, alpha cut
- [x] 1.4 Faces that could not be drawn are handed back and stay dirty

## 2. Shading

- [x] 2.1 Records carry the cube slot and radius; receivers compare 0.995·d / (R·k_r)
- [x] 2.2 Cached cube times Bevy's cube for skinned casters

## 3. Casters

- [x] 3.1 Static meshes in their lights' caster lists; changes and moves dirty the cubes
- [x] 3.2 Bevy's shadow map only for lights with skinned meshes linked
- [x] 3.3 Skinned meshes bounded by their joints

## 4. Checks

- [x] 4.1 exanima01 log room: shadows as before (frame mean 49.8 against the original's 49.1), about 200 FPS on a quiet machine
