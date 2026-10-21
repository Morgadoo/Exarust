# Design

## Context

Every routine was re-read in its instructions. The decompiler's text
misstated several details the corrections depend on: the order of float
sums, an index register dropped from an address (the navigation stamp's
ring), the branch a compare takes on NaN, and which word a field lives in.

## Decisions

- Pure functions over plain values: the golem and item-effect routines
  return what they write (`GolemChange`, `GoHome`, `ChargeOutcome`,
  `WornStep`), so the callers that do not exist yet (the AI, the character
  generator) can apply them.
- Reuse over copies: the alarm box uses `exa_physics::intersect`'s
  point-to-box distance and the plane heading of
  `exa_physics::body::locomotion`; the barricade's sector reuses the golem's
  sector index; the rigid shell copy reuses `shell_vertex` and
  `shell_indices`; the skinned copy keeps the existing marks, compact map and
  slots.
- The navigation stamp is the grid's own operation (`exa_world::nav`), kept
  as a patch so it can be taken back exactly; the live barricade stamps on
  the first tick a grid exists, since its grid is built after the level
  loads.
- Values whose role is not traced are named as such: the 20 cm value the
  character query receives, the third word the homeward callback sets, the
  body flag bit that exempts a body from waking.

## Open questions

- The character query's use of its 20 cm value: answered when the
  character's collision routines are read (REA).
- The fourth float of the node matrix's translation row and the 4×4 layout
  `LclMat` is built into: answered with the matrix builder (REA) when the
  shell program is ported.

## Docs

`docs/THAUMATURGY.md` (arcs, the shaped-force aura, live effects), the crate
READMEs; no `docs/formats/` page is involved.
