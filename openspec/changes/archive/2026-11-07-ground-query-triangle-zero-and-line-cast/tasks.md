## 1. Trace

- [x] 1.1 Re-check the Ground Query's index compare, the mesh callback and its float order against the assembly
- [x] 1.2 Trace the line cast, its filter and its three callers

## 2. Port

- [x] 2.1 The plain mesh ray with indices; the Ground Query drops triangle 0 of a floor node's first part
- [x] 2.2 Mark the first floor part from the model's groups and record it with the floor colliders
- [x] 2.3 The line cast, and the start of play onto its hit
- [x] 2.4 Unit tests

## 3. Check

- [x] 3.1 fmt, clippy -D warnings and tests for the touched crates on the worker; play build
- [x] 3.2 GPU grid before/after traces in exanima01, 02, 06 and arena1 with a weapon
- [x] 3.3 Update the exa_physics and exa_world READMEs
