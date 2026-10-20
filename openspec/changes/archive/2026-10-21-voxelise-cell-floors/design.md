# Design

Extends `docs/RENDERING.md` (Voxel GI). No `docs/formats/` page changes: the
generators make points at runtime and read no new data.

## Where the floor generator runs

The original's grid cell builder merges the cell's floor strips into one
`+Floor` mesh in the cell's space and runs the heightfield on it. Its edge
rule picks the x edges among the cell's quarters (−300…300 cm) and starts z
at −300 cm, so it only makes sense in that space. Exarust tags every grid
floor mesh with its cell (its centre on the grid, in centimetres) and its
transform into the cell's space. The GI collection groups the meshes by
cell, keeps the triangles in cell space, runs the generator once per cell
in centimetres and maps the points to the world. The block builds splat the
stored points; nothing is recomputed per rebuild.

Rays only test the faces that can take them: a face tested on x and z is
listed for the columns over its x and z range (one column wider), a steep
face for every column, in the faces' order. A test pins that every column
meets the same hit as with the whole list.

## Precision and order

- The plane distance sums `(a·x + b·y) + c·z − d`.
- The stored normal is `(b − a) × (c − a)`; the original normalises with the
  hardware's approximate reciprocal square root, Exarust exactly. The plane
  is rescaled by its y, so only rounding differs.
- Edge snaps take `v · 0.05` in single, subtract the double 0.1, round to
  even and multiply by 20 in integers.
- The surface step is `0.2 · p` in double and the near radius
  `s · √2 · 0.5` in double, both stored as singles.
- The downsample walks two rows per block row and one slice per block slice;
  odd row counts drift and that is kept.

## Voxel map layout

The constructor, destructor and array release fix the resolution, scale,
tag and the three arrays they free; the shape builder's grid fill fixes the
box, the counts, the one-cell border, Z-fastest indexing of 32-bit cell
words and the cell-centre formula; the sphere extraction fixes the 16-byte
sphere records and the two fill bytes per cell.

## Open questions

- The node's sphere query (kd-tree, collision level 1, or 2 for
  characters) and whether it is inclusive at the radius: REA on the kd-tree
  query and its callback, when the surface generator is wired to props.
- Collision triangles against render triangles for grid floors: a corpus
  comparison of a floor set's collision and render meshes (`cargo exa`).
- Which stage writes the fill bytes: REA on the solid fill and corner masks
  when the shape builders are ported.
- Retail agreement of the generated points: a capture of a `+Floor` mesh's
  point count in a running game.
