# Design

## Two modes, one state

`SunFit` keeps the half extent, the light-space centre, the up-sun (`near_cm`) and down-sun (`far_cm`) reach, the softness and the caster softness `c'`. `fit` (terrain levels) sets the reaches to ±8000 cm; `whole_scene` sets them from the scene. `matrices` builds Bevy's cascade from both reaches, so one code path serves both modes.

The original's light frame puts the smaller light-space z towards the sun and its box-to-NDC builder maps that side to −1, the depth test being "less": the up-sun reach is the original's first box term (the half extent) and the down-sun one the far field.

## Floating point

- The minimum raise is `comiss min, h` with "above": `min > h`; NaN never raises.
- The far field goes through the engine's max helper, one `maxss xmm0, xmm1`: the first operand when greater, else the second, so NaN gives the minimum. The decompiler's `a <= b ? b : a` would keep the NaN.
- The bias factor stores its first term single before reading it back widened; the nudge's `w` is single, its 0.3 compare in double, its ×0.01 in double.

## Wiring

`exa_world::sun_shadow` runs both modes each frame and writes one cascade per active camera. The nudge is subtracted (reverse-Z) at half size (`[0, 1]` depth) from `clip_from_cascade` only: Bevy draws casters with it and samples with `clip_from_world`, matching the original's separate caster and sampling matrices. The whole-scene cascade's bound is the scene's diameter, so receivers anywhere in the scene keep sampling it.

## The walk

`sun_casters::caster_draws` reproduces the render routine's loop over a caller-given caster list. The volume is pushed in place, as the callee writes into the same volume it reads. It is not driven: Bevy draws the cascade's casters with its own culling and Exarust has no scene-node caster list.
