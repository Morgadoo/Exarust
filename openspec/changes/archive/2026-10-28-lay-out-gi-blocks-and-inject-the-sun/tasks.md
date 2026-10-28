# Tasks

- [x] Rebase the lane onto main (already current).
- [x] Check the layout, lookup, sort, splat, dynamic region, uniforms, sun setup and shape loop against the assembly and the shipped shader sources.
- [x] Fix the depth-axis scale of light and voxel-light uniforms, the sun's world position, the no-sky sun sample direction and the sun's factor 3.
- [x] Fix the floored block lookup and its float half-grid edge, the focus-based fallback, the built-cell scan, the centred splat axis, the shape box rounding and the dirty-region rounding.
- [x] Publish the GI sun matrix only with a GI volume and only on change.
- [x] Tests: hand-worked bounds, lookups, splats, regions, shape boxes, uniforms.
- [x] Update `docs/RENDERING.md` and the crate READMEs; `docs/PLAN.md` is updated by the main session.
- [x] fmt, clippy `-D warnings` and tests for `exa_render` and `exa_world` on the worker.
- [x] `openspec validate --all --strict`
