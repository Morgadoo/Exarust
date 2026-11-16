## Why

Release of the held terrain lane (lane `terrainrel`, from the terraincheck
verdict "release with a change"): with blank terrain blocks left out of the
draws, a free view past the 44.4 m reach showed plant chunks standing over
missing ground, because Exarust drew every plant chunk whatever its block's
composite state.

## Evidence

REA (Exanima.exe 0.9.5), instructions read:

- The grass main draw hook, per inserted tile and per block node: the
  renderer's view mask against the node mask, then the block's composite
  state word with the blank bit (the same bit the lit terrain draw tests),
  then a non-empty instance list, the draw state and the instanced draw. A
  blank block's plants are not drawn.
- The grass shadow draw (sun branch and omni branch) tests the tile bits,
  the instance list and the node or light masks, never the composite state:
  a blank block's plants still cast.

## What Changes

- Each plant chunk names the 6 m block it stands on (`PlantBlock`).
- `blank_draw` lists the chunks of blank blocks with the blocks and takes
  them out of every camera's visible list; the sun's cascades and the cube
  faces keep them.
- `EXA_TERRAIN_DRAW_DEBUG=1` names, in the blank-block log, each camera
  that listed a blank block or chunk.

## Impact

- `exa_world::terrain` (`blank_draw`, `PlantBlock` in the tile spawn,
  `vegetation` notes).
- On screen: past the reach no plants stand over the void; within it
  nothing changes. Shadows unchanged.
- Lockstep traces unchanged (draw side only).
