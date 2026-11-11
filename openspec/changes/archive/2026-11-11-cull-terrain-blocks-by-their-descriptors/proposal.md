## Why

PLAN Phase 2 (outdoor levels). TStaticTerrain and RF_Terrain (unit routines) gap lane
(round 163, tag terrain3). Gaps named by the last ratings: the terrain's shadow draws
draw only blocks whose visible word is set, block bounds did not drive culling, the
height query's free-camera caller, the refresh pass, the composite lit draw's default,
and one small implementer refresh left unmapped.

## Evidence

REA (Exanima.exe), read in the assembly where the numbers matter:
- The two terrain shadow draws (every block of a tile for the sun's map, one block
  for a light's cube face) skip a block node whose external-draw bit is set, and draw
  only a node whose visible word has bit 0. Bit 0 is the main-view bit the scene's
  visible pass sets on a node it drew this frame (`0x1000001` together with the
  seen-by-some-view bit).
- A block node's culling box is its leaf descriptor: the 6 m block's centre and
  half extent across, and the min and max of the tile's 16 × 16 height bounds over
  its two by two bounds cells (finished, lifted vertices), the half height plus 1 cm.
- The palette refresh of the `.rft` loader (72 bytes) rebuilds the material rule tree
  from the chosen palette, then, when the terrain already has its grass object, calls
  its palette slot, which copies the first plant record's four parameters (the plant
  tint) when the palette has plants.
- The refresh pass (slot 18) is registered on the scene's clear hooks only; it runs
  when the scene is cleared, which Exarust's level teardown replaces.
- The height query's one shipped caller, the navigation camera's free tick, is already
  live (round 133's spectate camera).

## What Changes

- exa_world: `terrain::manager::block_box` / `block_box_in` (the leaf descriptor in
  metres); each spawned block carries its descriptor box as its `Aabb` (with
  `NoAutoAabb`), reaching over the skirt on a tile's edge.
- exa_world: `terrain::shadow_draw` takes the blocks the live scene's visible pass left
  undrawn out of the sun's cascade lists after Bevy's light check
  (`EXA_TERRAIN_SHADOW_ALL=1` keeps every block, to compare);
  `LiveScene::drawn_by_main`.
- docs/RENDERING.md (Terrain shadows), the exa_world README.

## Out of scope

The cube-face terrain draw keeps every linked block: Exarust's cached faces would keep
a block's missing shadow after it comes into view. The composite lit draw stays opt-in
(same picture, no faster). The composite selector's visibility still reads Bevy's
`ViewVisibility`, which the sun's check also sets.
