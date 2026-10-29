# Tasks

- [x] Rebase the lane onto main.
- [x] Check the lane's sector floor scan, terrain clamp, shaping order, corner offsets, program bindings and base ambient against the assembly and the executable's strings.
- [x] Revert the 29-column floor scan and the reordered shaping sum; redo the terrain clamp (lid's 5 overwritten, truncation, stored floor).
- [x] Correct the shadow sampler units (2 + i), the emission program's samplers and lookup order.
- [x] Update `docs/RENDERING.md` and the `exa_render` README.
- [x] fmt, clippy `-D warnings` and tests for `exa_render`, `exa_world` on the worker.
- [x] `openspec validate --all --strict`
