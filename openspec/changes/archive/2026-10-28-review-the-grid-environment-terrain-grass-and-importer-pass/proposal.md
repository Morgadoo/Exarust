# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), the gridenv paragraphs: the round-24
gridenv3 lane ported the plant scatter's precision, sort and instance
rotation, the grid's border fill, the placement tree override, a saved
placement's motion and AI-home bodies and the importer's handler registry.
The review checked each against the assembly and found the instance
rotation transposed, a wrong border corner word, an invented UV fill, and a
registry consulted where the original never consults it.

## What Changes

- Plants bake through the original's row-vector instance matrix
  (`(cos·s, 0, −sin·s)`, `(0, s, 0)`, `(sin·s, 0, cos·s)`, then the ground
  point) with its `(x·r0 + y·r1) + (z·r2 + t)` sums; grass and small-plant
  normals turn by the same rows and lose the scale; large plants take the
  terrain's blended vertex normal at their point
  (`vegetation::terrain_normal`, wired: the tile keeps its vertex normals).
- The vegetation density threshold is the double 0.35.
- The invented `(1/128, 0)` UV fill is removed (that loader step writes the
  +Y normals Exarust already sets).
- The grid border fill writes whole rows and columns, then the corners;
  the bottom-right corner is `0x13207700`; one-row, one-column and one-cell
  grids are filled too.
- Placement tree entries name direct children at every level, an empty name
  included; a record past the depth limit is still skipped by its stride.
- The node sub-chunk registry is consulted only in its reader mode, in place
  of the reader's own sub-chunk loop; root chunks never reach it.
- Saved placement bodies: the last chunk of a tag wins; the inferred
  layouts are labelled guesses.

## Evidence

REA and the objdump listing on the plant baker, the engine's sine/cosine
(sine to the first output), the packed row-vector transform and normal
helpers, the unit draw (2⁻³² double), the sort key (variant index against
the grass and grass-plus-small counts: 0, −1, 1 + mean height × scale), the
plant loader's normal writer and its per-library flags, the grid border
fill, the tree override (16-byte name compare, stride `(descendants + 1) ×
0x4c`) and the node reader's registry branch.

## Out of scope

The per-plant ground correction helper, wind, the per-chunk GI/shader key
and omni-shadow draws, the room sector/portal objects and omni shell faces,
the per-node darkness visibility words, the soft-body word's receiver, and
who reads the saved motion and AI-home bodies.
