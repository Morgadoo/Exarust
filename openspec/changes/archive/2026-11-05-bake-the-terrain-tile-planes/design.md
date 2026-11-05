# Design

Extends `docs/formats/TERRAIN.md` (Tile bake) and `docs/RENDERING.md`
(the terrain frame hook).

**Where the bake runs.** `terrain::tile_bake` runs on the tile worker
right after the masks merge, so the mesh, the ground query, the navigation
grid and the plants all read the baked planes. The frame hook still walks
the bake and read-back states so the insert happens on the original's
frame, but its bake step changes nothing on screen.

**Sampling.** The quads are axis-aligned, so each pixel's texcoords are a
linear function of its centre and the level of detail is constant per
pass and layer (`log2(side · f0 / 60)` plus the bind's bias). The material
maps are decoded through the texture loader with the terrain loader's word,
so their uploaded levels (the detail skip, a stored or Kaiser-generated
chain) and filter bits are the loader's.

**Open questions and how they were answered.**
- The maps' filter: the word has no linear bits and the terrain loader
  does not join the quality mask (REA, the loader call); nearest texel and
  level.
- The mask plane's encoding: sRGB writes are on for the mask pass onto an
  sRGB target (REA); the read-back's conversion is a driver rule, taken as
  stored values [inf].
- Blending: the base mode turns blending off; the group starting at layer
  4 switches to `ONE, SRC_ALPHA` and later groups keep it (REA).
