# Design

- `vegetation::bake` takes a `ground_normal` closure beside `place`; the
  terrain spawner passes `terrain_normal` over the tile's `n × n` vertex
  normals, cloned once per tile build before the skirt is appended (about
  0.7 MB per tile, freed with the built tile). Per plant this is one blend
  of four normals and one reciprocal square root, at load only; frame time
  is unchanged.
- `terrain_normal` keeps the original's corner weights, which run opposite
  to a plain bilinear blend; it uses `engine_math::reciprocal_sqrt` so the
  result is the processor's approximation, and gives +Y past the grid.
- The registry mode is `Scene::parse_with` with at least one handler: each
  node sub-chunk goes to the first handler of its tag and the reader's own
  sub-chunk handling is skipped, as the original's branch does. The game
  registers none, so the running reader is unchanged.
- `fill_borders` keeps the original's write order so degenerate grids end
  with the same words; only an empty grid is left alone.

On screen: plants keep their positions but each plant's yaw changes (the
previous pass turned them a quarter turn and mirrored the rotation); large
plants are lit like the ground under them. Check on GPU with
`exarust level arenao1` (and exanima01) against a retail shot of the same
camera: bushes' silhouettes and shading should match retail.
