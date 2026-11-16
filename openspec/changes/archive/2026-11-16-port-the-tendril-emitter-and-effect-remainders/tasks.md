## 1. Read the original

- [x] 1.1 The emitter's tick and frame, block by block against the assembly
- [x] 1.2 The node slots 17, 20, 28, 29 and their callers; slot 27's call sites
- [x] 1.3 The sector's test gates and the record's bits; the hand and the node frame in the tests and the draw
- [x] 1.4 The wisps' tick and strip, the blow creator and the state writers

## 2. Port and wire

- [x] 2.1 The test bits, the hand, the node frame, unturned sphere starts (`arcs`, `live`)
- [x] 2.2 The node lifecycle: attach at the build, add on the first step, remove on removal or despawn, the per-frame placement under a local-frame parent
- [x] 2.3 The shared strided record buffer
- [x] 2.4 The tick's and frame's float order and NaN branches
- [x] 2.5 The blow wisp's placement; the wisps' float order
- [x] 2.6 DEC-11 for the strip's scratch read and the respawn's endless draw

## 3. Tests and checks

- [x] 3.1 Live-system tests: only a record that asks shocks the bodies, a weapon's arcs are drawn in the weapon's frame, a phased blow's tendril enters the world placed; model tests for the spill and the unturned start
- [x] 3.2 fmt, clippy -D warnings, the workspace tests, openspec validate, the play build
- [ ] 3.3 GPU grid runs (owed; the card is on hold)
