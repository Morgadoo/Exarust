## Why

Roadmap task B4 (lane `tmesh`): the mesh class's per-triangle query
tests. The simulation answered the original's mesh query slots with its
own approximations: parry's point projection for the closest point, one
push floor for every mesh, NaN compares that reject where the instructions
go on, a rewritten push for a sweep that passes beside a face, an analytic
drop for the Height Query, and a one-sided front test for the overlap
query's two-sided slot.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly:

- The mesh constructor installs eight brute-force query slots; the
  kd-tree installer replaces seven of them with leaf walks over the same
  per-triangle tests. The slot callers: the level detection and the pair
  stage take the front sphere slot and the first swept slot, the
  placement query the front slot, the overlap query the two-sided slot,
  the Height Query and the picks the swept nearest slot, only the cloth
  the second swept slot; no code calls the segment slot.
- The closest point: the face record's region test on the two edges from
  the first corner (the far edge when a projection runs past), edge
  parameters by the processor's reciprocal estimate, MAXSS/MINSS clamps.
- The front sphere test: `0 ≤ s < r` (a NaN goes on), the closest point
  nearer than `r − floor`, the push to `r` or the normal times `r` under
  the floor; the floor is 1e-6 in the leaf test and 1e-10 in the
  brute-force one.
- The swept test (one routine for leaf and brute force): as described in
  `mesh_query`; past the end, the push scales the start's offset to the
  closest point by the end's gap and lifts it to the face.
- The swept nearest test: the closest point where the centre is one
  radius out, then `q·d − start·d − √(r² − p²)`, kept only inside
  `(0, length)`.
- The two-sided test: `|s| ≤ r` (a NaN goes on), `d² < r² − ε`.

## What changes

- `exa_physics::mesh_query` holds the per-triangle tests over the face
  record (`volume::Face::exact`, `Face::nearest_exact`), in centimetres.
- `contact::sphere_hit`, `swept_hit`, `drop_onto` and the new
  `sphere_touches` run them; the callers pass the floor of the form that
  runs them (`push_floor`): the pair stage by its part's tree, the level
  detection by the piece's node mesh (`StaticColliders::has_tree`, from
  the node's triangle count over all its meshes).
- The overlap query's mesh test is two-sided; the Height Query and the
  mesh trees' drop take the swept nearest test; the cloth's level test
  takes the record's closest point.
- [inf: the record's reciprocal square root and reciprocals are exact
  while DEC-7 is open.]
