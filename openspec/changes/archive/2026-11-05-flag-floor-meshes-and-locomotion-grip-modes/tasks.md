## 1. Trace

- [x] 1.1 Ground Query, floor cast, line cast and the mesh ray callback against the assembly
- [x] 1.2 The grip-mode setter's callers; the locomotion routine's writes
- [x] 1.3 List the floor-flagged nodes (`exa-tool corpus floor-nodes`)

## 2. Port

- [x] 2.1 Carry the floor bit to the mesh parts (models and tile grid floors)
- [x] 2.2 Record floor colliders; answer `Level::floor_piece` from them
- [x] 2.3 The sole floor's unordered compare
- [x] 2.4 The floor cast for the barricade
- [x] 2.5 The grip mode from motion starts and locomotion writes
- [x] 2.6 Unit tests

## 3. Check

- [x] 3.1 GPU grid: exanima01, arena, exanima02, exanima06 log their floor colliders and the barricade's cast
- [x] 3.2 Update the exa_physics, exa_world, exa_character and exa_formats READMEs and the motile-body map
- [x] 3.3 fmt, clippy -D warnings and tests on the worker; play build
