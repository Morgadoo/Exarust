# Design

Format page: [`docs/formats/RFC.md`](../../../docs/formats/RFC.md),
Editable meshes. Rendering notes: [`docs/RENDERING.md`](../../../docs/RENDERING.md),
Effect shells.

## Units

The original welds in centimetres; Exarust's positions are metres. The
key step (0.01 cm) becomes 1e-4 m, and the single-precision squared
distance is scaled by 10 000 before the double compare with 0.0001, so
the decision is the original's up to the rounding of the scaled values.
Texture coordinates have no unit and keep 0.001 and 1e-9.

## Faithful quirks kept

- The quicksort is the original's middle-pivot partition, not stable: the
  survivor of equal keys is the one it puts first.
- A later element is compared before its live flag is read, so a dead one
  out of reach can end the scan.
- Influence merging searches only the survivor's original slots, keeps the
  larger weight of a shared bone, and normalises every element with two or
  more influences by 1/sum in single precision.
- The UV sphere's fast sine rounds its quadrant to nearest, so past π/8 it
  evaluates the cosine polynomial on a remainder below −π/4.
- The sphere's strips are `segments` indices long; an odd count leaves the
  last index 0.

## Departures

- Unreferenced elements are dropped by the compaction (the original's
  dead-flag pass does the same); its uninitialised remap entry for a
  trailing dead element is not reproduced.
- The tangent normalise is exact where the original uses the processor's
  approximate reciprocal square root (the existing polyhedra helper).
- A refused (too large) mesh is converted through the editable form; the
  original converts the mesh as loaded, so the vertex order can differ.

## Open questions

- Whether the original's TGLGeometry buffer zeroes the pole tangents:
  answered only by a capture of the mind effects; Exarust writes zero.
- The FPC cosine against the platform's: a one-ulp difference is possible
  before the rounding to single.
