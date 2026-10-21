# Design

Rendering notes: [`docs/RENDERING.md`](../../../docs/RENDERING.md),
Character decomposition. Navigation: [`docs/NAVIGATION.md`](../../../docs/NAVIGATION.md).
Format page: [`docs/formats/RFC.md`](../../../docs/formats/RFC.md).

## Float semantics kept

- Ordered compares (`<`, `>`) on f32 and f64 are false for NaN, as the
  original's COMISS/COMISD plus jumps are; the clamp, the wrap and the
  large-step test of the preview therefore send a NaN to the easing branch.
- The engine's signum is the sign bit of a non-zero value (a NaN too) and
  0 for either zero.
- The preview's products by 0.7 and 0.05, its wrap compares and its yaw
  update run in double and are rounded to single once, as the original's.
- The centred slider multiplies by minus one, so its middle is a negative
  zero.

## Shared fast math

`exa_assets::polyhedra::fast_sin_cos` is the engine's sine and cosine pair
(one quadrant fold, two polynomials); `fast_sin` is its first half and the
curves' slerp and the preview's cursor direction use it.

## Cloth meshes

One mesh asset is shared by every copy of a model, and the cloth writes its
vertices, so `build_cloth_nodes` clones the asset per node and keeps the
clone in `ClothInstance`. A node whose mesh has no positions or indices is
marked `ClothSkipped` so it is not retried every frame.

## Not wired

The decomposition passes, the cloth helpers named in the proposal and the
player-control singleton have no caller in Exarust yet.
