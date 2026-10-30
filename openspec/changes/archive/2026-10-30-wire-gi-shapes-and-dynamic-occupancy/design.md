# Design

- Shape bodies run on the CPU as the tested reference: a shape's fragment
  reads only its uniforms and the voxel's position, so a per-voxel call
  computes what the quad-per-slice draw computes. GPU shapes wait for an
  effect that registers one. The bodies' constants are in the original's
  centimetres; `bMat` maps a voxel position into that space, so the
  cuboid constructor's frame is the translation by minus its centre.
  Products keep the shader's left-to-right order (`p.xy * 0.75 * 0.00375`).
- The dynamic update's frames live in `GiMoved { source, last }`, both set
  when the level's source collects the mesh. Exarust builds every block
  from that one collection, unlike the original's builder, which reads the
  nodes' current frames; a `synced` flag on each built block lets the
  update move every moved mesh from `source` to `last` once, the first time
  it sees the block.
- The gate is the original's: while a `GiBuild` exists, the update returns
  before touching any frame, so the old frame is kept for later.
- Cells are edited in place through `Arc::make_mut`; a volume that still
  shares them keeps its snapshot and the patch carries the change to the
  GPU. The patch list is cleared at the top of the update, so each patch
  is extracted and written once. A patch that arrives before its block's
  texture is on the GPU is lost until that block is uploaded whole again
  (an edge case: the block must be swapped in that very frame).
- The block test compares world boxes (the padded points' extent) where
  the original compares integer voxel boxes from the oriented bounds; this
  adaptation was already labelled.
- The share uses `(0.1f32 as f64 * 0.9) as f32`, as the MULSD/CVTSD2SS pair
  computes.
