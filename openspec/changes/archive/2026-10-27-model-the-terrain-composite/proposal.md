# Proposal

## Why

Phase 2 (static world) builds the terrain, but the original's texture
compositor, its block selection and its tile streaming bits were only partly
modelled, the palette's wet-floor source was parsed and never applied, and
the debug wireframe hook had no counterpart (TStaticTerrain and
FLX_Terrain_Texrender2, second pass).

## What Changes

- `terrain::composite` holds the compositor's numbers: the texture-detail
  levels and set sizes, the pools of 21 high and 43 low sets, the pass keys
  and groups of four, the per-layer pass values, the block quads,
  `TexPos`, and the block selector's state machine (keys, stable sort, the
  blank-first walk, low-set scan, high-set steal and trade, removal, the
  ring).
- The palette's wet-floor source above 0.4 turns on the lit program's
  wet-floor term in `terrain.wesl`.
- `terrain::stream` keeps the tile streaming bits and the frame hook's walk.
- `terrain::wireframe` draws each block's triangle edges under
  `EXA_TERRAIN_WIRE`.
- The level build sizes the pools and records each tile's streaming bits.

Evidence: the instructions of the terrain draw (the `> 0.4` test that adds
the WetFloor environment bit), the decoded `shadetrn_f` program, the pool
allocator, the block selector and its set hand-out helper, the frame hook,
the worker, `TexPos`, the wireframe hook and the ground query's bilinear sum.

Out of scope: offscreen composite textures and their GPU passes (Exarust
layers the materials per pixel), a live streamer after a load, the
per-block light-distance intersection, the scene's Wetness path, the
detail texture (absent from the install), grass wind.

## Capabilities

### New Capabilities

### Modified Capabilities

- `terrain`: the composite's sets, passes and selection, the wet floor, the
  tile streaming bits and the wireframe overlay.
