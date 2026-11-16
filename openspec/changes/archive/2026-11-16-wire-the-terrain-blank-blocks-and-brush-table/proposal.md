## Why

Roadmap task B4 with B1 (lane `terrain`): the static terrain (TStaticTerrain)
and its unit routines. Findings F1 and F3 of the audits: the original neither
draws nor casts terrain shadows from a block whose composite state is blank,
while Exarust drew and cast every block; the static terrain model had its
activation and deactivation the wrong way round, and the selection's separate
removal reset the block state, which the original does not. The block light
rank ran a second copy of the engine's convex-volume clearance, and the rule
children and painted layers resolved by a plain name match instead of the
original's brush table.

## Evidence

REA (Exanima.exe 0.9.5), checked against the instructions where constants,
order or branches matter:

- The lit terrain draw: per inserted tile, each block whose composite state
  has the blank bit is skipped before the view-mask test and the block's
  render state; the cube/GI branch sits after the same test.
- The tile shadow draw and the one-block cube-face draw: the same blank test,
  then the main-view bit.
- The block selection: the out-of-reach branch writes the wanted and blank
  state; its key pass reads the main-view bit and, for a hidden block, calls
  the convex-volume clearance once per light of the block's room, keeping
  the smallest (NaN never wins), rounded and biased by 0x10000.
- The clearance: centre plane value and support summed x, y, then z, the
  normal taken into the box's frame, the half extents ORed with the normal's
  sign bits, a NaN plane value going on and a NaN clearance never winning.
- The separate removal routine: swap-remove, set back to its pool, state word
  untouched; its callers test the list index first.
- The palette refresh: rules then materials inserted into the sorted table,
  each rule's two children then looked up in it; the tile evaluation looks
  its layers up the same way.
- The manager constructor scales the height rules of the rule table it has
  just created (count 0), so the scaling changes nothing.
- The static terrain's activation (inactive bit cleared, frame, scene, draw
  and wireframe hooks inserted, the 100 ms release registered once, worker
  resumed, extensions activated) and deactivation (the reverse, then the
  composite sets, pools, selection and block nodes freed).

## What Changes

- Blank blocks leave every camera's visible list, the sun cascades and the
  cube faces (`EXA_TERRAIN_DRAW_BLANK=1` turns it off).
- The selection reads the live scene's main-view bit for the blocks it holds.
- The block light rank calls `exa_physics::intersect::outside_distance`.
- Rule children and painted layers resolve through the brush table.
- The selection's separate removal leaves the state word alone.
- The static terrain model gets `activate` and `deactivate`.

## Impact

- `exa_world::terrain` (`blank_draw`, `shadow_draw`, `live`, `composite`,
  `blend`, `stream`, `plugins`), `exa_physics::intersect`.
- On screen: no terrain draws or casts past 44.4 m from the point 10 m ahead
  of the camera; a block in view without a set stays undrawn.
- Lockstep traces are unchanged (draw-side only; the selection orders bakes,
  which the simulation never reads).
