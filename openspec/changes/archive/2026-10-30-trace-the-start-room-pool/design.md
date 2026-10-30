# Design

## Context

`docs/RENDERING.md` (Lights) and `docs/formats/RFC.md` (light node) describe the light record and its fall-off. The comparison captures and measurements are in `context/artifacts/2026-10-30-retail-cube/` and `context/artifacts/2026-10-30-lightpool/`.

## Decisions

- **The range rule lives in the parser.** `exa_formats::rfc::ranged(flags2, radius_cm)` decides `Light::ranged`, so the renderer, the shadow links, the flood and the GI all follow the original's activation from one place.
- **No new light.** The pool is the level's own light, which Exarust already draws. Porting a player light or a view light would add light the original does not have.
- **Debug options instead of probes.** Light listing, starting without the torch and laying a torch on the floor are useful for every later retail comparison, so they are kept as flag-gated options.

## Open questions and how they were answered

- What makes the pool? The REA disassembly of the player light's maker, frame and equipment switch shows the switch is on only for an item whose body has special kind 1, and the retail save's character database holds the background-0 letter and no helm. `--debug-lights` then listed the two level lights at the spawn, and a no-torch render reproduced the pool.
- Does the fill light fall off? The light activation sets the attenuation to 0.8/R and the far plane to R only with the range flag and R > 0. The multiplier's ε test gates a different field, which the forward shading does not read.
- Why are the periphery's neutral levels lower? Still open. Zeroing the fill light in Exarust blacks out the periphery entirely, so Exarust's GI adds nothing there. Retail's extra might be GI accumulation (the base ambient is 3.95e-5 a step, below one 8-bit sRGB step). Settling it needs a capture of the retail light volume.
