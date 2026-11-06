## Context

The original's mesh is one node mesh whose triangle array covers every
section; Exarust builds one collider per drawn mesh group. Group order and
the triangles within a group keep the file's order, so the part built from
the group with `first_index == 0` has the node mesh's triangle 0 as its own
triangle 0. A `_null` first group is a pick mesh in Exarust and is left out.

## Decisions

- The rapier collider keeps its build order, so a triangle's BVH index is
  its index in the part; the query sorts the candidates by index to walk
  them as the callback does (ties go to the lower index).
- The determinant epsilon is the engine's in cm²; Exarust's metres scale it
  by 1e-4.
- The original tests in the mesh's own space; Exarust's colliders are in
  world space (the transform's rounding differs by an ulp or so).
- The start cast keeps the stored start when it misses (the original keeps
  the lifted point, which would drop the player from 10 m).
