# Proposal

## Why

Exarust drew a terrain tile as one mesh (one per room in a grid level),
built its geometry and material masks on the main thread inside the load's
frame budget, and had no world height query. The original draws one mesh
per 6 m block from a patch every block shares, builds tiles on a worker
thread, answers "where is the ground" from the tiles it built, and streams
blocks into composites by distance rules. Its lit terrain program builds
the normal map's frame by a cross product, which Exarust approximated with
two projected axes. The change advances docs/PLAN.md **Phase 2 — Static
world** (terrain) and the Phase 5 terrain look.

## What Changes

- Each 6 m block is its own mesh, built from the original's shared 30 × 30
  cell patch (serpentine rows, quads split along their low-to-high
  diagonal), flattened into the tile's 241-vertex rows and drawn from the
  block's first vertex; the patch builder's LOD shift and its 16-bit
  indices are modelled.
- A tile's geometry and masks are built on the async compute pool, and the
  worker starts the next tile while the main thread spawns one.
- `TerrainGround`: the original's world height query (engine floor, slot
  `tz · width + tx`, block bits, the byte-wise hole mask, bilinear heights
  in its order, no ground at or below −100 m).
- `terrain::stream`: the 60 m priority ranks (round to even, one-sided
  lowest/highest scan), the composite focus 10 m ahead snapped to a block
  centre, the 44.4 m / 15 m squared-centimetre cut-offs with NaN kept, the
  189-block list, the 21/43 pools and the 3 × 3 gather.
- Lit program: the normal map and the parallax view vector in the frame
  +X projected and crossed with the normal; roughness 1 − gloss with no
  floor (a mirror's singular point kept at 0); specular masked by the red
  of the diffuse colour after `MatDif`.
- The palette's header floats at 24 and 28 are parsed.

## Capabilities

### New Capabilities

### Modified Capabilities

- `terrain`: block meshes, the tile worker, the height query, the streaming
  rules and the lit program's frame.

## Impact

- `exa_world::terrain` (`patch`, `ground`, `stream`, `frame`, `mod`,
  `terrain.wesl`, `vegetation`), `exa_world::nav` (the ground tile moves to
  `terrain::ground`), `exa_formats::terrain` (palette floats),
  `exa_render::shading` (the GGX divisor's floor).
- Docs: `docs/formats/TERRAIN.md`, `docs/RENDERING.md`, the crate READMEs.
- Evidence: REA disassembly of TStaticTerrain (patch builder and flattener,
  Init, the height slot and its tile helper, the priority slot, the block
  selection, the 3 × 3 gather, the block draw's vertex offset) and the
  engine's round-down helper; the original's `shadetrn_v`/`shadetrn_f`,
  `terrain_nrm` and `terrain_col` programs; the shipped palettes (tiling
  factors, header floats, gloss factors up to 1.2).

## Out of scope

- Offscreen composite textures, their 21/43 set pools, the light-distance
  sort and set assignment, and the composite copy passes: Exarust evaluates
  the composite per pixel.
- Live streaming states, the one-tile-per-frame hook and tile unloading:
  the load builds every tile behind its loading screen.
- The wet-floor roughness term (recorded, not applied), the `trndtl01d`
  detail texture and the wireframe overlay.
- Wiring `TerrainGround` into gameplay (no caller yet).
