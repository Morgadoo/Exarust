# Tasks

- [x] Node kinds, registration by kind, batch insertion, locked sectors, change bits and the membership rebuild in `exa_world::scene_nodes`.
- [x] Subtree moves, child validation, alias removal, frames in sector space.
- [x] Sector bounds and scene bounds with the original's seeds, caps, centres and sum orders.
- [x] The 6 m cell and spill mask shared with `light_flood`; the cell handler rule.
- [x] Classification, the hidden and visible passes, the doorway test, inherited bits, fades and the scissor.
- [x] The light flood's directional fill.
- [x] Tests with hand-worked numbers for each rule.
- [x] Corpus check: none needed (the model reads no game data); the real-data tests of `exa_world`, `exa_render` and `exa_game` pass on the worker.
- [x] `crates/exa_world/README.md`. No `docs/formats/` page changes; `docs/PLAN.md` is updated by the integrator.
- [x] fmt, clippy -D warnings and tests for `exa_world`, `exa_render` and `exa_game` on the worker.
- [x] `openspec validate --all --strict`.
