# Design

## Context

`exa_formats::rfc` decodes scene nodes, static meshes and materials (see
`docs/formats/RFC.md`). REA on the level loader established:

- `0x3DED` tile grid: `u32 n` + `name[16]×n` wall sets, `u32 n` + `name[16]×n`
  floor sets, `u32 width, height, u32 legacy_marker, u32 skip`, then three
  planes of `width×height` `u32` values stored one after another. A legacy
  marker of 600 triggers a conversion of planes B and C (older files).
  Plane A packs four 4-bit edge codes (low 16 bits) and four corner codes
  (high 16 bits) per cell; the engine's default border fill shows the code
  rotating one nibble per edge.
- `0x3DE10000` props: `u32 count`, then per prop a chunk (`0x3DE10100`):
  `u32 flags`, `name[16]` object, `f32[12]` placement (cm), `u32 parent`,
  `u32 ?`, then optional sub-chunks (`0x3DE0EC00`, `0x3DE0ECAC`,
  `0x3DE0ECDB`).

## Goals / Non-Goals

**Goals:** levels visible in Bevy with props in place; resources shared;
nothing fatal on missing data.

**Non-Goals:** grid geometry, terrain, exact lights, characters, gameplay.

## Decisions

- **Decoding stays in `exa_formats`.** `Scene` gains `grid` and `props`.
- **Conversion in `exa_assets`.** A `ModelCache` resource maps
  `(pack, entry)` to decoded nodes with mesh and material handles; textures
  are cached by name. CPU-decoded RGBA8 for now; direct BC upload is a later
  optimisation.
- **Spawning in `exa_world`.** A prop spawns one parent entity at its
  placement transform with children for the object's nodes (node transforms
  composed in the object's own hierarchy).
- **Transforms.** Stored rows are row-vector basis rows; Bevy's `Mat4` is
  column-major with column vectors, so the stored rows become Bevy columns
  unchanged.
- **Lighting.** Until light nodes are decoded, a dim ambient light plus a
  point light on every prop whose name contains `light`, `torch` or `candle`
  stands in for the original lighting.
- **Camera.** Free-fly (WASD + mouse look, Shift faster) spawned above the
  level centre.

## Open reverse-engineering questions

| question | how |
|---|---|
| meaning of the prop `flags` and the second word | REA on the prop reader's users |
| grid planes B and C | next change (grid builder), REA on the builder |

## Risks / Trade-offs

- Thousands of props × CPU-decoded textures may load slowly; caching keeps
  each texture decoded once, and BC upload comes later.
- Software rendering in the development container makes screenshots slow;
  real performance is measured on the GPU host.
