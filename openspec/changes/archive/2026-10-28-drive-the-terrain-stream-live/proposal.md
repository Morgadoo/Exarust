# Proposal

## Why

Phase 2 (static world) builds the terrain, and the previous pass modelled
the compositor, the block selection and the tile streaming bits as tested
functions only. The terrain's frame hook and block selection now run in the
game after a level loads (TStaticTerrain and FLX_Terrain_Texrender2, third
pass).

## What Changes

- A built tile spawns hidden and is marked built once its entity exists;
  the frame hook's insert shows it, and the block-until-loaded wait flushes
  every tile before the level shows.
- `terrain::live::TerrainStreaming` runs the frame hook and the block
  selection every frame from the active camera's look-ahead focus; the
  selection is skipped while the refresh delay counts down; an unloaded
  tile's blocks leave the selection and hand their sets back.
- A hidden, in-reach block ranks by the oriented-box / light-plane distance.
- A failed low-set hand-out leaves the frame unchanged.
- The priority slot ranks only the selected list; it is not run every frame.
- The compositor's pass list, mask and height passes, quads, sampler units,
  pool layout and `TexPos`'s third component are modelled.
- `exa_render::terrain_composite` can host the composite sets as render
  targets, opt-in with `EXA_TERRAIN_SETS=1`.

Evidence: the instructions of the block selector, the light-distance plane
walk and its vector-by-matrix helper, the priority slot, the frame hook, the
program builder, the height pass and `TexPos`.

Out of scope: a GPU bake into the composite sets and drawing them (the
per-pixel shader supplies the same content), per-room light lists, the
caller of the priority slot, per-node sector relinking, tile reloads after
the load, the detail texture (absent from the install).

## Capabilities

### New Capabilities

### Modified Capabilities

- `terrain`: the live frame hook and selection, the light distance, the
  pass bindings and the hosted sets.
