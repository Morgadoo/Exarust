# Tasks

- [x] Trace the factory's assembly, the mesh append, the weld, the render form, the material assignment and the paint job's targets and pass order with REA, checking the part matrix, scale and UV map against the assembly.
- [x] `exa_assets::weapon`: assemble one merged mesh (part frame, atlas UVs, regroup, weld, render form, bounds) and take the striking extents' denominator from it; cache components with their stored box and split flags.
- [x] `exa_assets::atlas` and `exa_assets::paint`: paint every region into one shared atlas in the original's pass order; `factory_atlas_material` makes the weapon's one material.
- [x] `exa-tool factory`: write a factory weapon's atlas and its UV overlay.
- [x] Corpus checks (skipped without the game): every shipped factory assembles into one node with one mesh, finite normals and texture coordinates; every chosen component has a shade texture; overlapping chosen regions listed; a real save's striking extents compared (110 of 143).
- [x] GPU runs: `exanima01 --weapon "@UBM Spetum A1"` and `--weapon "@GlmHalbrd A"` (fixed camera) and `exanima06` golems, before (round 55) and after.
- [x] docs/formats/OTHER.md and the exa_assets and exa_tool READMEs updated; docs/PLAN.md is left to the orchestrator.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker (`make check` scope).
