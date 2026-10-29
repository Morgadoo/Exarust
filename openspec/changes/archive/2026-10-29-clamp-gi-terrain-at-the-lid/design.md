# Design

- Terrain: `TerrainColumns::apply` follows the fill step by step: `k` is
  the height in voxels cast with truncation (`as i32`; non-finite heights
  are never stored, so NaN cannot reach it), skipped below zero, the
  surface byte written only for `k` < D − 1 and cast through `i32` so a
  negative fraction wraps as the original's byte store does. Terrain runs
  before the points, so storing `k` as the column floor equals the
  original's plain store.
- Sector floors keep the full 30 × 30 scan already in Exarust: the loop
  counters start at −1 and are tested after the body, so the bound 29 is
  inclusive. The lane read it as exclusive.
- The shaping sum keeps dy² + (dz² + dx²); the block-ranking sum of two
  squares commutes exactly and is left as it was.
- `GiPass::layout` is a tested table: the WESL compute pass binds its own
  resources, so nothing reads it at run time. It records the original's
  units so a GL-faithful path or a debug view can follow them.
- Corner offsets: the double path equals the f32 one for the shipped
  sizes (256, 64, 128) and differs for others (37 in the test).
