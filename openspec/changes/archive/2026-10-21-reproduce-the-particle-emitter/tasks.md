# Tasks

- [x] Port the particle emitter's rules to `exa_world::particles`, with
  tests on hand-worked values (scaling, derived values, spawn at rest, ring
  law, distance cap, extra ageing, pops, key lerp, light density, diamond
  widths, fast arc cosine, one-shot and finite stops).
- [x] Review every rule against the original's assembly; fix the spawn
  speed, the source frame, the first-tick motion, the arithmetic and the
  rounding.
- [x] Bake the six key tables from the raw records with the load-time
  scaling, and scale the 0.5–1 bed by the whole size.
- [x] Face the flames from the camera in centimetres, carry the held
  torch's fire, remove finished fires, skip hidden flames' quads and cap
  the catch-up ticks.
- [x] Correct the arc presets, the record's fields, the charge step, the
  device light and the ribbon's UVs.
- [x] Update `docs/RENDERING.md` and the `exa_world` and `exa_render`
  READMEs. `docs/PLAN.md` and the component map are left to the
  integrator.
- [x] No corpus fields change; the corpus checks still skip when
  `EXANIMA_DIR` is unset.
- [x] `make check` for the touched crates (tests and clippy on the worker).
