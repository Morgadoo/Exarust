# Tasks

- [x] Reproduce the seam on arenao3 (seed 15) and locate it with the GI debug view: the camera block's edge at Z 4.4 m, cube ambient beyond it.
- [x] Check the original's per-object lookup, its texture-0 fallback, the surface program's lack of a box test and the block texture's wrap with REA.
- [x] Publish and step every resident block; give each its sector marks, lights and margin copies.
- [x] Pick the block per object in the surface and terrain shaders; plant chunks at their block's centre.
- [x] Tests: the pick rule, the slots, the neighbour textures, the layout's pick, the published neighbours, the terrain uniform layout.
- [x] Verify on the GPU: arenao3 seam gone, no new seam, exanima01 unchanged, frame time held.
- [x] docs/RENDERING.md (docs/PLAN.md left to the integrator).
- [x] fmt, clippy -D warnings and tests for `exa_render` and `exa_world` on the worker.
