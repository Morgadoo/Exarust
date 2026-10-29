# Design

Extends `docs/PLAYER.md` (The hand, The door) and the mechanisms section of
`crates/exa_physics/README.md`; no `docs/formats/` page changes (the door
record and the body chunk were already documented in
`docs/formats/RFC.md` and `docs/formats/OTHER.md`).

## The leaf as a body

The original's door is a rigid body like any hung prop, created by the
importer's factory hook, then set up when added to the world. Exarust
registers the leaf through the existing hung-prop path (`exa_world::bodies`,
`Placement::Door`) and adds a `PendingDoor` on its root; a fixed-step system
builds `DoorBody` once the `Ragdoll` exists, so registration order is the
props' own.

The door frame mixes the hinge axis read from the body's rest points with
the stop's tied point in the parent node's frame, as the original does; the
two agree for any node turned about the vertical, which every shipped door
is. The angle turns the stop's offset from the body's origin into the
parent node's frame (the transpose of its rotation, as the original's
row-vector product does) and then into the door frame.

The drive moves the stop point to the point at the driven angle on the
stop's radius about the body's origin, spread by the stop's weights. The
original applies it in the body's step between integration and the link
passes; Exarust runs it after the solver's tick (the links meet it a tick
later; the velocity it adds is the same).

## The hand's link

The press passes slack 1.0 (stored squared) and strength 0.003, then sets
the link's flags to the iteration limit alone with last pass 2. The world
link's plain slack branch therefore applies: (len − 1 cm)/len · 0.003 of the
gap in each of three passes. The old `DOOR_LINK_SLACK = 0.003` named the
strength as the slack and is gone.

## Spheres

A door leaf keeps all its ~1 900 stored spheres, as the original reads them
all. The original meets them only when the leaf is the more mobile side
(against something heavier than 0.05); a character's limbs bring their own
spheres against the leaf's mesh. Exarust's parts carry no meshes yet, so
the fallback tests the leaf's spheres; `SphereGrid` buckets them so the
fallback stays cheap, visiting candidates in stored order with the push
re-read after a hit (tested equal to the plain loop).

## Open questions

- The door's sounds and navigation report: their callers are known (the
  step reports the sounds; the passage report goes to the scene's
  navigation object); answered when the audio and grid sides take them.
- The door's mesh as the pair's mesh side: needs the loader to hand node
  meshes over (`Ragdoll::set_part_triangles`).
- Frame time on GPU: verified by the integrator (`play exanima01 --perf-log
  5`).
