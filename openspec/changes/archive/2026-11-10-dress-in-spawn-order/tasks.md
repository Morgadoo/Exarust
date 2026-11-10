## 1. Changes

- [x] 1.1 `DressAtOnce`: the dress in one frame under `--fixed-frames`, fits and paints built in place
- [x] 1.2 Dress, hair and off-hand passes in spawn order
- [x] 1.3 `BridgeHalf::register`: bridged resources registered in the frame's world; test `a_bridged_resource_is_held_by_the_frames_hold` (fails without it)

## 2. Checks

- [x] 2.1 Duel with `--weapon ub05ssworda`, 1500 frames, 2 grids x 2 scenes: four traces 0000370a... byte-identical, body ids included; entity census (`EXA_ENTITY_TRACE=1`) identical up to the frame a resource's value is first seen
- [x] 2.2 Plain duel (`play arena --opponent --frames 1500`), 2 grids x 2 scenes: four traces 873e2c63... byte-identical
- [ ] 2.3 exanima01 x2: GPU Out of Memory (external vLLM engine)
- [x] 2.4 exa_character, exa_physics and exarust READMEs; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of exa_character, exa_physics and exarust on the worker
