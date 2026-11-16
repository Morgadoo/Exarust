## 1. Read the original

- [x] 1.1 The link builder, its two neighbour helpers and the set-up's list order and half rests, against the assembly
- [x] 1.2 The constraint pass's edge branch and its switch; the integrate loop's order
- [x] 1.3 The wind hook, the tile sample, the frame builder and the integrate's wind block, with their constants
- [x] 1.4 The integrate slot's rustle block and the draw update's lerp and normal rebuild

## 2. Port and wire

- [x] 2.1 The link walk (per mesh at load) and the particles in element order
- [x] 2.2 The full-correction edge pass and the integrate order
- [x] 2.3 The wind hook once a simulation step from the scene's preset wind; the cloths blown in the world's frame
- [x] 2.4 The rustle math, its message from the cloth stage and its voice in the audio
- [x] 2.5 The draw update between the last two ticks with the flat normals rebuilt

## 3. Tests and checks

- [x] 3.1 Unit tests worked from the assembly: the walk on a quad and a strip, the edge pass, the integrate order, the wind hook, the tile sample, the wind push and its double floor, the rustle
- [x] 3.2 Live-system tests: the scene's wind blowing the cloths, the rustle and its stop, the interpolated write-back with normals
- [x] 3.3 The walk over every shipped cloth mesh (EXANIMA_DIR)
- [x] 3.4 Crate READMEs; fmt, clippy -D warnings, workspace tests, openspec validate
