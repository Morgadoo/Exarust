# Design

- The slice centre follows the shipped vertex shader: the quad's r is
  −0.5/depth and the layer offset is subtracted, the fragment flips the
  sign. The compute pass keeps one invocation per voxel and samples the
  same coordinate; `voxel_uv`/`voxel_height` mirror it on the CPU for tests.
- The light block keeps three vec4s per light as the original's
  `Light[3i..3i+2]`; the shaping sits in x, y, z with the sun flag in w
  (the original has it in y, z, w and a separate program branch for the
  sun). The original binds one cube sampler per light index; Exarust's one
  cube array needs a slot, kept in a parallel `cubes` array.
- A test lays `gi.wesl` out with naga and compares `GiLight`,
  `GiVoxelLight`, `GiGroup` and `GiParams` with the CPU's `ShaderType`
  sizes, so a resized block cannot reach the GPU mis-sized.
- The cube faces store distance / R. An unranged light's compare scale is
  tiny, so its taps stay lit; the original draws its faces against the
  same 3.4e38 far plane, so its taps probably shade wherever geometry is
  (a labelled guess, not reproduced).
- Surfaces take the half-voxel lift through the material origin
  (`GiTextures::sample_origin`); terrain and grass use
  `sample_origin_floor`. The lit-material builder's flags2 `0x800000`
  exception is not modelled (the reader does not keep that bit's meaning).
- Cost: one extra uniform array read per light per voxel; the uniform
  block shrinks by about 9 KiB.
