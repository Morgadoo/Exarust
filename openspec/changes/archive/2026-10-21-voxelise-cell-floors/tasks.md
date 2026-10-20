# Tasks

- [x] Port the voxel map, the down-ray query, the floor heightfield and its downsample.
- [x] Port the surface generator, its clearing pass, the greedy cover and the per-cell variant.
- [x] Check every routine against the pseudo-code and the assembly: constants, comparisons, rounding, precision, loop and output order, bounds.
- [x] Merge each grid cell's floor meshes and run the heightfield once per cell in cell space at collection.
- [x] Tests: map layout and release, stored plane, plane-distance order, edge snap, odd-row downsample, double-precision step and radius, per-column face lists, cell merging.
- [x] Real-installation check: the GI collection logs each level's floor point count (`GI source: … floor points`), run in the app; no corpus command reads generated points, skipped without `EXANIMA_DIR`.
- [x] Update `docs/RENDERING.md` and the crate READMEs; `docs/PLAN.md` is updated by the main session.
- [x] `make check` equivalent: `cargo fmt --all`, clippy `-D warnings` and tests for `exa_physics`, `exa_world`, `exa_render`.
- [x] `openspec validate --all --strict`
