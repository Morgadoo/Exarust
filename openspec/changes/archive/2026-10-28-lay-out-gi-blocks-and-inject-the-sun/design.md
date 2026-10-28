# Design

- One resident light texture stands in for the original's nine. The camera's
  own block is drawn; a surface across a block edge samples the active
  block and clamps. The fallback when the camera's block has no geometry
  is the original's residency order (distance from the focus 20 m ahead to
  each block's 24 m half extent).
- The injection stays one compute pass. It writes each voxel and samples
  where the original's layered quad does: x and y at texel centres, z on
  the slice boundary; light positions and distances use the height over
  the block's width (the original's `ZTrn`). Uniform positions are
  (X, Z, height) over the block's X and Z extents and its width.
- The sun reads Bevy's fitted sun map (reverse-Z, greater-or-equal
  comparison) through the scene camera's first cascade matrix, published
  only while a GI volume exists and only when it changes. Point lights read
  the cached cube pool with the surface shaders' flipped Z.
- Built-cell bounds, shape box, margin copy and dynamic region are plain
  functions with the original's numbers, tested by hand-worked cases.
