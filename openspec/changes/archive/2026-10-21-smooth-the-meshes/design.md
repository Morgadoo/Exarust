# Design

Format page: [`docs/formats/RFC.md`](../../../docs/formats/RFC.md), Editable
meshes.

## Model

- `EditMesh` mirrors the original's mesh in editable form: channels (key,
  corner triples), streams (id, format word, channel, values), material
  groups (face ranges, split flags), triangle flags and edge bits, the
  subdivision level, the face-to-centre-child map, and the state the
  original keeps between edits: the incidence and the smoothing groups.
  `from_mesh` rebuilds it from a decoded mesh (channel keys and stream
  words as shipped); `render` converts it back and leaves it editable in
  the new face order, so it can be subdivided again as the original can.
- The incidence lists each element's corners last first. The splits
  (smoothing groups, material, seams, winding) are one swap-gather pattern:
  the seed is the range's next corner and every matching corner is swapped
  into the next free place.
- The vertex-cache order works on unified vertices and their incidence
  runs. Its score tables are built once from the fast curves.
- `subdivide` follows the original's step order; each rule is a method of
  `Rule` (incidence, creases, weights).
- Geometry stays in metres; the original works in centimetres. The rules
  are linear and the bounds floors are converted; results equal the
  original's up to float rounding of the unit scale.

## Open questions (answered)

- Which meshes the original smooths at run time: REA on the callers. The
  CharExt hair while the creation scene's flag is set, every master mesh
  of a scene for the debug command and the item preview; the GI wrappers
  in the same unit are not subdivision.
- What the triangle flags hold: corpus (`CharExt.rpk`, single smoothing
  bits in the low 16 bits) and REA on the smoothing-group builder.
- Which groups split at texture seams and windings: REA on the material
  loader's flag bits (any slot but the reflection map; a loaded normal
  map).
- The fast curves: REA on the instructions (scalar SSE, round-to-nearest
  conversion), pinned bit for bit.

## Deviations

- Polyhedron midpoints use an exact reciprocal square root; the original
  uses the processor's approximation.
- Skin data of a subdivided mesh is padded with zero weights (the original
  leaves the stream short). Each interpolated stream keeps its own weights
  (the original shares and frees one buffer; one stream ships).
- Unreferenced position elements decode as the origin.

## Docs

`RFE_MeshSubdivide`, `RFE_MeshTools`, `RF_Mesh` and `RF_Polyhedra` rise;
the integrator rates them from the returned notes.
