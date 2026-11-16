## Why

Roadmap task B4 (lane `charanim`, triage lane L09a characters and
animation). The ledger left the cloth's blood painter uncalled, the mesh
painter drawing per render vertex in world space, and the object keyframe
tracks' clock and curve reported as started only by the F7 debug key. Three
readings of the lane were also open: a golem's stance factor, the skin
controller's fade-in alpha and the motion loader's child sort.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- The wound paint of a body's head: for each child of the head's node, a
  cloth controller is painted by the cloth's painter, a mesh node by the
  mesh painter (deferred). The cloth painter moves the point back by half
  the travel, normalises the travel, and takes the point alone into the
  node's frame; it paints the drawn vertices in place.
- The mesh painter takes the centre and the travel into the mesh's frame,
  loops over the position elements, draws a random number for each element
  inside the wound and writes every vertex of an element through the index
  map.
- The object use starts the tracks of the used node and of the nodes under
  it before its sound (already wired in round 235).
- The character init's golem branch writes the stance factor (0.75), the
  damping ratio (0.95), the clip rate and the pin factor (1); the join that
  every race branch reaches writes the race record's stance, damping ratio
  and clip rate again, and the feral step its pin factor. The shipped golem
  records store a stance of 1.
- The skin controller's per-frame deform computes the 600-tick fade-in
  alpha in a register, calls a buffer bind that takes no float, and never
  reads the value.
- The motion parts' transfer pairs parts by the first two characters of the
  node name (the bone number); the shipped skeleton's numbers are unique.

## What changes

- `exa_character::blood`: a cloth (long hair) under the head is painted in
  place on the mesh its simulation moves (`paint_cloth`), no longer copied
  away from the cloth; plain meshes are painted in their own frame, once
  per position element (`paint_mesh`).
- `exa_physics::cloth::paint_blood`: takes the node frame for the point.
- Tests: the used secret switch plays its brick's track through the live
  systems; a golem keeps its record's stance; the shipped skeleton's bone
  numbers are unique.
- Docs: `docs/RENDERING.md` (Blood), the fade-in alpha's note in
  `exa_anim::skin`.
