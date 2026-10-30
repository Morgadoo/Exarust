# Proposal

## Why

The rendertail2 lane (Phase 5 — Visual and audio fidelity, with the
Phase 2 scene-node model, see docs/PLAN.md) added the original's light
activation values, the alias copy, the geometry node's activation and
master hand-off, the content helpers, the terrain block activation, the
kd-tree background builder, the smoke pass draws, the character icon
renderer's set-up and a material's own environment cube. Its review found
several rules read from pseudo-code or invented: GI and range gates taken
from the wrong flag word, an alias copy of record fields the original does
not copy, an invented fade scale, an order-keeping hand-off where the
original swaps, a builder that refused work after a reset, duplicated
helpers (luma, the image-format registry, scene-content wrappers) and an
invented terrain block lookup.

Evidence: REA (Ghidra) reads and assembly of the light constructor,
activation and its derivation steps, the alias sync, the geometry
constructor, shadow-scale and activation routines, the instance add,
remove and master hand-off, the content helpers' detach and attach, the
format registry add, the terrain block bounds and activation, the kd-tree
builder's push, thread body, hand-back, drain, reset and query install,
the smoke draw, and the icon renderer's set-up and resize.

## What Changes

- `exa_world::light_source`: `DerivedLight` (the intensity's epsilon clamp,
  the node-word GI and second-colour gates, `√I·M·3`, luma, reach and the
  `0.8/R` falloff) and `AliasValues` (the derived values and the node
  word's bits 16–23).
- `exa_world::scene_nodes`: the alias sync stores those values; detach
  drops a light's alias count only; attach appends without unlinking.
- `exa_world::geometry_node` (new): part flags, activation in order, the
  instance lists and the swap-order hand-off.
- `exa_world::terrain`: `TerrainBlock::activation`.
- `exa_physics::kdtree`: `BackgroundBuilder` (discard and idle flags) and
  `QueryTable`.
- `exa_render::smoke`: `ProgramCache`, `pass_draws`; `exa_render::icon_render`
  (new) with `exa_ui::icon_atlas::IconImage::from_icon_pixels`.
- `exa_assets::model`: a material's reflection layer binds its own cube,
  only when it loads as a cube.

## Out of scope

- Driving the renderer from `DerivedLight` or the alias values, spawning
  `GeometryState` for models, a live kd-builder thread, a live smoke mesh
  and the character icon render job.
- The smoke program key's extra bits (the global flag and node `0x8000`),
  the icon preset's other environment fields, the original's full scene
  clear (builder wait, holder scenes) and the release's held-node walk.
