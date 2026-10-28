# Proposal

## Why

Phase 2 (static world), terrain milestone: the original shows its terrain
through per-block composite sets, baked one block a frame and read by its
`shadetrn` program. The previous pass hosted the sets as render targets
but nothing wrote into or read from them, and a hidden block was ranked
against every light in the level rather than its room's (TStaticTerrain
and FLX_Terrain_Texrender2, fourth pass).

## What Changes

- The renderer bakes a block into its set: the compositor's normal,
  colour, gloss and half-size displacement passes in its order, each a
  full-target quad over the block, with the per-pixel draw's layer
  stacking (now a shared shader library), one block a frame.
- A block whose bake has run draws from its set with the original's
  `TexPos` coordinate and four-step parallax over the set's height; it
  returns to the per-pixel draw when it loses the set.
- The colour target is stored as sRGB so its 8 bits are gamma-encoded like
  the original's RGB8.
- A hidden block ranks against its room's light list (the room's own
  lights and those its portals carry in).
- `EXA_SCENE_SEED` fixes the outdoor day's seed for side-by-side checks.

All composite work stays opt-in with `EXA_TERRAIN_SETS=1`.

Evidence: the terrain draw's per-block set binding and `TexPos` upload,
the compositor driver's pass order and targets, the shipped `shadetrn`
vertex and fragment programs (composite coordinate and parallax), the
room manager's light-add hook and portal propagation; a GPU run of
`arenao1` comparing the composite draw with the per-pixel one.

Out of scope: the 32² mask/height scratch textures (the bake stacks
directly), GPU blits for set copies (a copied block is baked again), the
caller of the priority slot, tile reloads after the load, the absent
detail texture and the 3D noise texture.

## Capabilities

### New Capabilities

### Modified Capabilities

- `terrain`: the composite bake and draw, and per-room light ranking.
