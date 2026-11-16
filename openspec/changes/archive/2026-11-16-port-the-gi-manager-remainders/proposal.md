## Why

Roadmap task B4 (lane `gimanager`): the voxel GI manager's open ledger
rows. The scene clear's shape-list reset was missing, the injection took
the shapes in ECS order, the voxel lights and shapes were summed into the
propagation and encoded once, the splat measured points from the block's
corner in metres, the terrain columns came from mesh triangles rather than
the tile's vertex grid, and the dynamic update boxed a moving mesh by its
points rather than its node's bounds.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly where order and constants
matter:

- The scene clear calls the manager's virtual slot that sets the shape
  count to zero; the manager's slot that marks every built block dirty has
  no call site (the slot is introduced by the AO-map base, whose stub is
  abstract; the manager is held only by its global and the renderer's
  field, and no call through either reaches that slot).
- The injection's draw order: the groups, then every voxel light as its
  own draw (blending on), then every listed shape as its own draw, both
  over their boxes whatever the groups drew; the emissive programs write
  alpha 0. The target is SRGB8_ALPHA8 with framebuffer sRGB on, so each
  draw's sum is encoded.
- The splat: the frame's rows times 1/fc, the translation times 1/fc less
  0.5 plus the block's whole-voxel offset, the point through the engine's
  transform `(x·r0 + y·r1) + (z·r2 + t)`, the centred variant's +0.5 after
  it, truncation.
- The terrain fill: the quad's anti-diagonal vertices of the tile's
  241-wide grid, `((a + b)·0.5)/fc + b4`, over the tile's present sectors.
- The dynamic update: each frame's world box from the node's bounds
  through the frame's absolute rows, `(|r0|·hx + |r1|·hy) + |r2|·hz`, over
  fc less and plus 1.5, `cvtss2si`; a block is edited when its whole-voxel
  range meets the union.
- The variant: the neighbour weighting depends on light 0 being kind 2;
  every light is kind 3 except the manager's sky light and the scene's
  sun (multiplier 0).

## What changes

- `exa_render::gi_shape::GiShapes::clear`, called by
  `exa_world::gi_volume::unload_level` (the level teardown).
- `exa_render::gi::gather_lights` takes the shapes in `GiShapes` order.
- `gi.wesl`: `blend_srgb8` per voxel light and per shape, after the
  groups, over dark voxels too; the stored alpha quantised;
  `exa_render::gi::blend_srgb8` mirrors it.
- `exa_world::gi_volume::Occupancy::splat_mesh` for the build's meshes and
  floor cells and for the dynamic update.
- `exa_world::gi_volume::GiTerrainHeights` on each terrain tile and
  `TerrainColumns::add_tile`.
- `exa_world::gi_volume::GiBounds` on each GI mesh and `voxel_box` for the
  dynamic update; `Occupancy::region_of_box`.

## Impact

Pictures: the voxel lights' and shapes' light per pass (a faint light no
longer adds up across overlapping lights), terrain floors one slice off
where a quad's diagonals differ, lattice points on voxel boundaries.
