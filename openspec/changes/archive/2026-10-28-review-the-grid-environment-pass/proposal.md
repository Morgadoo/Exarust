# Proposal

## Why

docs/PLAN.md, Phase 2 (static world), tile-grid dungeons and the room
manager: the gridenv2 lane (RFE_GridEnvs unit routines, TGridEnvManager)
added the grid's set catalog, the special floor table fix, link grouping,
the control-side door query, the active-room flag, the room light lists and
the shell query. Its review checked each rule against the executable's
instructions and data and found several that differed.

## What Changes

- Special tiles: types 6–9 keep their centre floor (the executable's table
  holds 1 at indices 5–8), as the lane found; the start-up pass turns all
  ten masks (its counter runs 0 through 9), so type 10 turns too (the lane
  had stopped at nine).
- Room flood: links back into a room are removed by moving the last link
  into the hole, and links are grouped by target room only after every room
  exists (the lane grouped inside each room's flood, while later rooms were
  still unassigned).
- Control-side door query: no room checks (two points in no room reach),
  and the first link into the second point's cell decides.
- Active-room flag: the working state's low bit (faded in, or wanted again
  mid fade-out); the debug reset does not set it.
- Room light lists: a room that owns at least one light passes its whole
  list on (the original searches its list from the end for an entry it
  owns), not only a room whose last entry is its own.
- Shell query: the engine's one-sided mesh ray test (origin, direction,
  limit; determinant at least the f32 epsilon; 0 < t < limit), with the
  segment form scaled by the reciprocal of the length.
- The exanima01 walkable-floor numbers follow the floor-table fix.

## Evidence

The executable's start-up rotation loop, the per-cell builder's floor table
read, the flood's removal and grouping loops, the door, active-flag, light
and shell query routines, the engine's mesh ray test and its epsilon, all
read from the disassembly; the special tables read from the PE image.

## Out of scope

The omni-light ceiling shell, the `TSector` object graph, the per-cell
merged floor mesh, the queries' callers (picking under the cursor, the
segment test), and the first registration of a `Terrain` floor set (the
original's floor registration loads a new set at once; recorded as open in
docs/formats/RFC.md).
