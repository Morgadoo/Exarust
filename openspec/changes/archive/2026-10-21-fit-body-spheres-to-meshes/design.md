# Design

No `docs/formats/` page changes: the fit makes bodies at runtime and reads no
new data (the stored body chunk is described in `docs/formats/RFC.md`, Rigid
bodies). Rules: `crates/exa_physics/README.md`.

## Reusing the voxel map

The original's routines all take the voxeliser's map (box, counts, longest
axis, the bordered Z-fastest cell words, the two fill bytes per cell, the
sphere list, the 0.02 resolution and unit scale, and the tag that names the
mesh tree's per-thread stack). `exa_physics::voxel::VoxelMap` already holds
that layout, so the fit fills it in place: `resize` makes the grid,
`cells_mut` takes the fills, `set_fill` the corner masks and `set_spheres` the
extraction. The corner offset is `(half/n) · resolution · scale` from the map.

## Rounding and precision

- Longest axis: `round(half · resolution)` by the processor's conversion
  (nearest, ties to even), kept as the low 32 bits of a 64-bit result.
- Short axes: the round helper sets the rounding mode toward +∞ for one
  conversion, so `n · half / longest` rounds up.
- Wide-query radii: `(half/n) · √2 · 0.5` and `· 0.75` in double, stored as
  singles; without the flag `(half/n) · 1`.
- The +axis ray length adds the double 0.2.
- Summary: index sums `(i + sum) − ½`, centre `((sum/count)/n − ½) · half · 1`,
  mass `count · ((2hz/nz) · ((2hy/ny) · (2hx/nx)))`. The original's sums start
  from stale stack words; in the builders' order they are tiny denormals that
  vanish against any index, so they are taken as zero.
- Extraction cell sizes are `(half/n) · 2` (not the grid's `2·half/n`); a
  candidate's radius `r/(r · 0.01 + 1) · cell` is taken in double.

## Mesh queries

The original's sphere query walks the kd-tree's leaves overlapping the sphere
and runs a per-triangle test; its ray query walks the leaves along the ray
and stops at the first hit. Both answer a yes/no here, so a walk that visits
every face that can answer gives the same result. `volume::Mesh` keeps a
bounding-volume tree whose node boxes are widened by a thousandth of their
size and of their coordinates (far past the rounding of a nearest point or a
hit point); a test checks it against every face on a lattice of points that
lands on faces, edges and radii.

The triangle record keeps the normal scaled by the approximate reciprocal
square root, its plane constant, the first edge's squared length and the
approximate reciprocal of the edges' Gram determinant. The nearest point is
the original's region test (not the exact closest point), its edge
parameters taken with the approximate reciprocal and clamped by the
processor's MAXSS/MINSS operand order. Exarust takes the same instructions
on x86-64, so a cell exactly at a query's radius falls as in the original on
the same processor; the unit tests use geometry clear of the radius.

## Real-installation check

Every `Objlib` body node is fitted from its mesh in the file's centimetres
(the words behind the parsed metres, found in the file) over its stored box,
and its spheres compared bit for bit, in the file's metres, with the stored
list. The stored lists come from two versions of the fit, one making
candidate spheres `r` cells across instead of `r/(1 + 0.01 r)`, and most were
made with the voxeliser's wide-query flag; the check takes the version and
flag that match. Of the 1503 shipped bodies 1060 match: 526 with the builders' defaults,
the rest with the wide flag, the other resolution or a depth of 8.

## Open questions

- The bodies no option set reproduces: meshes edited after their bodies were
  made, or a third version of the fit. A corpus comparison of the unmatched
  bodies' cell counts against their stored corner-sphere radii would tell.
- Which settings raise the builders' quality flag and the wide-query flag:
  REA on their writers.
- Whether the kd-tree's leaf assignment can miss a face a sphere touches:
  REA on the tree build.
