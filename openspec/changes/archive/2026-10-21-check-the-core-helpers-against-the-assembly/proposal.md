# Proposal

## Why

A DeepSeek lane ported seven of the original's smaller units: the text and
number helpers, the engine's maths unit, the prelude unit (game tips, actor
templates, practice opponent), the object manager's scene decisions, the
terrain material routines, the item randomiser's remaining routines and the
save package's open, reset, probe and checkpoint copies. Much of it was
ported from the decompiler's pseudo-code and several rules were wrong or
invented. This change checks every routine against its instructions and
records what the units now reproduce.

## What Changes

- `exa_physics::engine_math`: one shared implementation of the engine's
  fast maths (the UI's helpers forward to it, the body's twist helpers are
  re-exported from it), with the sine-cosine mask pick, the atan2 NaN
  branch, the look basis normalised with `RSQRTSS`, component-wise row
  scaling, the inverse's pivot rule and the runtime library's arcsine and
  arctangent rules.
- `exa_formats::text`: the formatting and parsing helpers, with the float
  formatter's 64-bit conversions and keep-zeros flag, the 1-based substring
  search, the double-precision fraction sum and the hex padding.
- `exa_game::prelude`: the tips popup's two tables and its draw, the actor
  template reader with and without its cache, the practice opponent's
  traits, team and reaction delay; the viewer's `--opponent` uses them.
- `exa_world::db_objects`: the scene tree, node flags, kind words, material
  rules, masters and release; only the placeholder of an operative record
  without a model is spawned.
- `exa_world::terrain::blend`: the brush table's binary search, the
  eleven-brush limit (wired into the tile build), the height-rule scaling,
  the byte channels, the kernel, the merge and the packing.
- `exa_items::randomise`: the grade gate, the row application and the
  world-load walk.
- `exa_formats::save_package` and `exa_game::slots`: a package open writes
  nothing and keeps the bytes read; the checkpoint copies by name.

## Capabilities

### New Capabilities

- `engine-maths`: the engine's fast maths and small linear algebra.
- `text-helpers`: the engine's text and number helpers.

### Modified Capabilities

- `save-games`: package opening.
- `terrain`, `object-database`, `item-instances`, `practice-ladder`: new
  requirements.

## Impact

- Crates: `exa_physics`, `exa_ui`, `exa_formats`, `exa_game`, `exa_world`,
  `exa_items`, `exa_assets` (a global `*`-material hide is reverted),
  `exarust` (the `--opponent` template).
- Docs: `docs/formats/SAVES.md`, `docs/formats/TERRAIN.md`, crate READMEs.
- Evidence: REA (Ghidra 12.1.4) on Exanima.exe: the pack's assembly of the
  seven units and the callees they use (the runtime library's name compare
  and lower-case copy, `DeleteFileA`, the vector length, the kernel
  normaliser, the condition and palette-byte setters, the array RTTI that
  sizes the tip tables, the practice spawn's fixed transform). The actor
  library check reads `EXANIMA_DIR` and skips without it.
- Out of scope: the tip texts (retail data), the uncached actor reader that
  builds a template with its object chunks, the practice opponent's fixed
  spot in Exarust's world, a live writable object database for the
  randomiser and the scene decisions, the material record builder of the
  terrain unit, the FPS readout's 250 ms refresh, and the other copies of
  the engine's sine-cosine and logarithm elsewhere in the tree.

## Phase

Phase 3 (bodies: the engine maths the body twist uses), Phase 4 (items: the
randomiser), Phase 2 (static world: terrain materials and placed objects)
and Phase 7 (game modes: save packages, tips, practice opponent).
