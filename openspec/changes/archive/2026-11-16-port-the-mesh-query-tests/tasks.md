## 1. Read the original

- [x] 1.1 The mesh constructor's slots, the tree installer's, and each slot's callers
- [x] 1.2 The closest point, the front, swept, swept nearest and two-sided tests, branch by branch with their unordered compares and constants

## 2. Port and wire

- [x] 2.1 The tests in `mesh_query` over the face record, exact reciprocals (DEC-7 open)
- [x] 2.2 The level detection, the pair stage, the placement and overlap queries, the Height Query, the mesh trees and the cloth's level test on them
- [x] 2.3 Level pieces record their node mesh's tree form

## 3. Tests and checks

- [x] 3.1 Unit tests of each branch; a live-system test of the static build's tree form
- [x] 3.2 fmt, clippy -D warnings, the workspace tests, openspec validate, the play build
- [x] 3.3 Determinism runs and a before/after pair on the build worker's GPU
