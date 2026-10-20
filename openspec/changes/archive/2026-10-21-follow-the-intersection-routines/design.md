# Design

## Behaviour

Every routine is re-expressed from the assembly, not the pseudo-code: the decompiler prints commutative sums in an arbitrary association and models the processor's minimum and maximum as an ordered comparison, which differs on NaN.

- **Sums.** Dot products and squared lengths add x and y, then z. The normalisation sums x² + z², then y², and leaves a zero vector zero. The four-component plane–light product adds (x·x′ + z·z′) and (y·y′ + w·w′).
- **Minimum and maximum.** The first operand when strictly smaller (larger), otherwise the second. Slab tests fold the per-axis entries and exits x, y, z with them.
- **Apart answers.** The sign bit of reach² − distance²: touching overlaps.
- **Branches.** The side the processor's unordered result takes. The polygon walks treat a NaN distance as outside; a NaN projection on a segment measures from its start.
- **Oriented-box distance.** Overhangs are compared as integer bit patterns.
- **Polygon walk.** From the last vertex's side: outside, find the entry, keep the inner run, stop at the exit; inside, keep the run to the exit, skip the outer run, keep the rest from the entry. A vertex on the plane is kept once and is its own crossing.
- **Pyramid.** Face i passes through p[i − 1], the apex and p[i], its plane refreshed from them.
- **Volume cut.** Each face keeps its plane and its walked polygon (three points or more). Cuts are oriented entry → exit when the face's last vertex is outside, exit → entry when inside. The cap chain starts from the first cut, turned round when the first face's last vertex is inside, and takes the first remaining cut with an end within √5 cm of the chain's end, moving the last cut into its place. A chain whose ends are 0.01 cm or more apart (compared in double precision) is extended from its start; otherwise its closing point is dropped. A cap of three points or more becomes the last face.
- **Shadow volume.** Faces with plane · light ≤ 0 are open. For each open face and each other face, the first edge of the open face matching an edge of the other reversed (within √5 cm) and at least 1 cm long gives a side plane, normal light × edge, through the edge, with a polygon extruded 10 000 cm along −light. The faces not open follow without polygons.
- **Cube-face mask.** Six 45° diagonals and, per face, four (diagonal, sign) side planes from the shipped table; a face is left out when −sign × (centre · d − light · d) exceeds the radius on any of them.

## Model

Pure functions on Bevy vectors in `exa_physics::intersect`; `ConvexVolume` of `Plane`s (normal, offset, polygon). The cube mask is `exa_render::cube_shadows::sphere_faces`, its bits put in Bevy's face order by axis.

## Open questions

- The processor's approximate reciprocal and reciprocal square root differ by processor; Exarust keeps correctly rounded ones (agreement to about 12 bits on sweeps, normals and the oriented-box slabs). Settled as a documented approximation.
- The original reads past a list for a polygon with no vertices, a point set of fewer than two points and a face with fewer than three vertices; Exarust guards those and documents the guards.
- Callers: the body-pair meet needs each part's mesh box, the directional shadow pass and the portal walk do not exist yet; each is its own change.

## Docs

`docs/RENDERING.md` (Cached cube shadows: the face-mask rule). No `docs/formats/` page changes.
