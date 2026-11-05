# Tasks

- [x] Read the bake driver, both passes, the blend helper, the read-back hand-off, the worker, the frame hook and the terrain loader's map word in REA.
- [x] Run the tile mask and height passes on the CPU with the GPU's rules (`terrain::tile_bake`), with tests for the quads, groups, NaN clamp, sRGB write and level of detail.
- [x] Feed the height plane to the vertices and the packed green to the plants through the hand-off; drop the per-vertex stand-in.
- [x] Fix the frame hook's bake and read-back bits and the passes' blend flag.
- [x] Verify against the installation: shipped nav goldens (arenao1, exanima01) updated, terrain corpus tests pass.
- [x] Update `docs/formats/TERRAIN.md`, `docs/RENDERING.md` and the exa_world README; `docs/PLAN.md` is updated by the reviewer.
- [x] fmt, clippy -D warnings and tests for exa_world; `openspec validate --all --strict`.
