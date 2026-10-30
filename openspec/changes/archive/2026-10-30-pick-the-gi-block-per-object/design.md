# Design

## The seam

`play arenao3` builds four blocks. The camera's block spans Z 4.4 m to
55.6 m; the ground short of Z = 4.4 m lies in the block at Z −43.6 m. The
surface shader tested whether the fragment lay inside the camera's block
and otherwise took the environment cube at mip 5 × the level ambient. Under
the stands' shadow the cube side measured 35.8 and the GI side 20.4 (mean
luma, 1920 × 1080 capture, rows 712–744): a 23.7-level step on one row.
The GI side was also dark near the edge because the camera block's margins
took copies only from blocks the camera had visited, so they stayed black
and absorbed the light that reached them.

Of the candidate causes: the sampled block's bounds and the out-of-volume
fallback are the seam itself; the margins and the single stepping block are
why no neighbour light existed to pick; the sky-floor column texture is per
block and plays no part at the edge.

## Per-object pick

The original picks per object. Exarust's materials are shared assets, so
the pick runs in the fragment shader from the instance's own transform: the
mesh's world origin (`world_from_local` translation) is floored over 48 m
from the grid's low edge, and the slot whose block holds that cell is
sampled. This is constant over a draw, as the original's uniform is. The
material carries nine slots: the light texture of the camera's block in
slot 0 and the resident neighbours in 1..8, each slot's sample origin and
block-grid cell, and the grid's edge. With no matching slot the sample is
(0, 0, 0, 1), the original's texture 0. A block off any grid (the
focus-centred block of a grid with no built cell) keeps the old behaviour:
every object samples it.

Terrain parts are drawn with the tile's transform, so the terrain shader
takes the centre of the 6 m terrain block under the fragment (the tile is
eight blocks a side) as the object's position. Plant chunks were drawn at
the tile's origin; they now stand at their block's centre with their
vertices offset, so the surface pick reads the block's position.

The switch over the nine textures sits inside the tap loop (one sample
per case), not around it, so the shader is not nine copies of the loop.

## Stepping the residents

`publish_neighbour_volumes` lists the residency's stepping blocks other
than the camera's, in layout order, and leaves the resource alone while the
list names the same builds (the floors' allocation identifies a build), so
the surfaces are rebound only when the resident set changes.
`create_neighbour_textures` makes each listed block's textures through the
texture cache, so a block the camera left keeps its light. The render world
builds each block's parameter buffer and bind group, reuses its margin copy
bind groups while both textures live, and runs one step per block per frame
after the camera block's. A new neighbour owes the volume's warm-up; the
neighbours share 16 extra steps a frame.

## Cost

arenao3 (four blocks, 128 slices) with `--perf-log`: `exa_gi` 0.28 →
0.97 ms on the GPU, frame 10.4–10.7 → 10.9–11.0 ms. exanima01 (nine of
twenty blocks resident, 64 slices) is bound by the main thread's physics;
its frame time stayed within the run-to-run spread.

## Open questions

- Which point the lit-material builder passes for a node (its origin or
  its bounds' centre) was not traced to the field; Exarust uses the mesh
  entity's origin [guess].
- The original's nine texture objects are handed between blocks with their
  old light; Exarust keeps a block's own light and starts a new block
  black with a warm-up [adaptation].
