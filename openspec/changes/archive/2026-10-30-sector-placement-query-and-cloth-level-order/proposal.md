# Proposal

## Why

Round 39's simulation-sector pass (docs/PLAN.md, Phase 3 — Bodies:
skeleton, physics, control) reproduces three more rules of the original's
grid sector: the placement overlap query's nine-cell scan, the
object-body stage's body mask, and the order in which a simulated object's
static contacts are gathered relative to its step. The review found the
port of the placement query measured the sphere centres against
world-aligned boxes and skipped a NaN distance, and named the skipped
state bit wrongly; both are corrected here.

Evidence: the grid sector step and the placement overlap query read in
REA (Ghidra listing) and the objdump listing; the cloth class's virtual
table (its step slot, called in the sector's object loop before the body
and static contact slots, and its post-collision slot between them); the
point-to-oriented-box distance routine (point into the body's frame,
integer compare of each axis against the half extent, overhangs summed
x, y, z).

## What Changes

- `exa_physics::sector`: `PlacementBody`, `placement_offered` and
  `placement_scan` (the nine cells around the probe's, the sphere gate,
  the two oriented mesh-box gates through `mesh_box_distance2` with a NaN
  passing, the same-root group filter, the other-root layer filter, the
  disabled skip, the lower pair key first). Tested; no live caller.
- `exa_physics::cloth`: `step` splits into `step_before_level` and
  `finish_step` around the level contact.
- `exa_world::cloth`: the stage gathers a cloth's static records and
  triangles from its bound after its step (live), and offers no body
  contacts from a disabled body (live).

## Out of scope

- Gathering a cloth's body candidates after its step (the original steps
  every object, re-buckets it, and only then gathers bodies; Exarust
  still gathers bodies from the bound before the step).
- A live caller of the placement query, and the padded test's scene mesh
  query.
- Per-sector body, static and object lists; Exarust's solver is
  world-wide.
