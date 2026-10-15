## Why

A living body knocked over by a blow could sink through the floor and drift
away. In `exarust play exanima01 --shove 120`, 17 of 33 shoved bodies ended
with the pelvis below the floor within about 130 frames. Two defects in
Exarust caused it, and neither is how the original works:

- Level contacts used the nearest point of a two-sided triangle mesh. Once a
  sphere's centre slipped under a floor, the "push out" pointed down and
  carried it through. There was no swept test, so a fast tick could carry a
  centre past the floor, and contacts were solved only after all the link
  iterations, so links dragged resting limbs under before any contact ran.
- The muscle drive spread each frame's pose change over its ticks as a
  rotation about the world's origin. Far from the origin, a fall turned that
  into particle steps of 37–80 m per tick.

The original keeps every body part out of level triangles with one-sided
faces, swept tests and contacts stored for the next tick's link iterations.

Phase 3 (Bodies), milestone M3 "standing man": a character stumbles when
pushed, falls and gets up.

## What Changes

- Level contacts as the original does them: faces push from their front
  only; a sphere that moved more than 3.2 mm in a tick is swept from where
  the last tick left it; a push moves the part at once and is kept as one of
  at most four contacts per part, which hold their planes through the next
  tick's 20 link iterations; the original's friction law runs 8 times after
  detection and again in those iterations. The 8 separate contact passes and
  the 2r push clamp go.
- The drive spreads a frame's pose change as a motion of each part about
  itself; a fall restarts the targets instead of counting as motion; a
  fallen body's muscles act inside the body: they neither feed its own
  motion back nor drag its fall, and can neither move its centre of mass
  nor turn it.
- A mirrored level mesh keeps its faces' front side.

Evidence: reverse engineering of `Exanima.exe` 0.9.5 (body–level contact
detection, the per-triangle sphere and swept tests, the four-contact
manifold, the world friction pass, the order of one simulation tick), and
per-tick probes of shoved bodies in the running Exarust.

Out of scope: contacts between bodies; the original's own pose drive (its
carry and pelvis-height servo); foot planting and per-tick foot friction;
inverted-tetrahedron repair; the −50 m kill plane; impact sounds from the
level; spheres voxelised from the mesh; which level nodes count as
collidable.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: level contacts and friction; limp bodies, muscles,
  ragdoll stability and sleeping bodies updated to match.

## Impact

- `exa_physics` (`body.rs`, new `contact.rs`, `lib.rs`), `docs/formats/RSQ.md`
  (Physical body), `docs/PLAN.md`, `crates/exa_physics/README.md`.
