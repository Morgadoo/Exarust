# Tasks

- [x] Verify the outdoor curve, the cube factor and their order in the dumped shader library, and the switch, `Ambient.w` and the outdoor cube in the binary (REA, objdump listing).
- [x] Settle the GI sky light's colour activation (written and read raw; no linearising step).
- [x] Port the outdoor shading into `shading.wesl`, the world material and the terrain (reflections, hair, fog on the light before the cube).
- [x] `GiOutdoor`: the flag on lit and terrain materials and the level's cube (`genref02r` under a sky).
- [x] Tests with the original's numbers (`exa_render::gi_ambient`), shader constant and flag checks.
- [x] GPU captures against the pre-change build, with and without GI (noon, storm with a forced strike, interior).
- [x] docs/RENDERING.md and the exa_render README (docs/PLAN.md left to the integrator).
- [x] fmt, clippy -D warnings and tests for `exa_render`, `exa_world` and `exa_assets` on the worker.
