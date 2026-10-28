# Proposal

## Why

Three manager rules were still missing from the voxel GI: the per-frame
residency that ranks the level's blocks, builds the next one and moves
light textures between them; the thin-mesh flag the splat reads (its
producer was unknown, so `GiThinMesh` had none); and the level record's
flag that lets the light steps draw the top slice. The assembly now gives
all three: the residency's distance, sort, build pick and texture
handoff; the mesh set-up that derives a surface's thin byte from its
material's class bit `0x4000` or an opacity layer above 0.75; and the
injection's instance count, `D` or `D − 1` by the environment record's
flag `0x20`.

## What Changes

- `exa_world::gi_residency::GiResidency` reproduces the manager's block
  list: nine textures for the first blocks, built blocks measured from the
  focus (box overshoot per axis, clamped at 0 unless NaN, squared, Z
  first), a stable insertion sort, the first unbuilt or dirty block built
  next, and the farthest textured block beyond the nine handing its
  texture to a built near block that has none.
- The world ranks the blocks every frame, builds the block the residency
  names next in the background while the camera's block stands, and keeps
  every built occupancy for the level. Cached blocks are keyed by layout
  index (they were keyed by block-grid slot and looked up by index).
- `Material::is_thin_for_gi`; the model reader derives `ModelNode::gi_thin`
  from the first surface's material and the level spawn adds
  `GiThinMesh`.
- `exa_formats::environment::GI_TOP_SLICE`; the GI uniform's spare
  `sun_flags.z` lane carries it and the shader draws the top slice when
  it is set (`drawn_slices`).

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: GI block residency; GI light texture handoff; GI thin meshes; GI top slice.

## Impact

- `exa_world::gi_residency` (new), `exa_world::gi_volume`, `exa_world`
  level spawn and load; `exa_assets::model`; `exa_formats::material`,
  `exa_formats::environment`; `exa_render::gi` (`gi.rs`, `gi.wesl`).
- `docs/RENDERING.md` (Voxel GI), `docs/formats/RFC.md` (environment
  flags), crate READMEs.
- Uniform layout unchanged (a spare lane is used); the layout tests stay.
- Background builds use the async compute pool one block at a time; the
  renderer still draws one block, so GPU work per frame is unchanged.

## Out of scope

Nine resident light textures on the GPU and the per-object texture
choice, the margin blit pass, the shapes' fragment bodies, the lit
material's `0x800000` sample exception, the dynamic-update wiring, and an
unranged light's cube far plane.

## Phase

Phase 2, static world: the voxel GI (lighting milestone).
