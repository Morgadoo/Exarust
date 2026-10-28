# Tasks

- [x] Shared layer-stacking shader library; the per-pixel terrain draw uses it.
- [x] Bake queue and render-world bake pass in the compositor's order, one block a frame.
- [x] Composite draw planner and material swap; terrain shader composite path with `TexPos` and parallax.
- [x] sRGB colour target; material and bake uniforms laid out against their WGSL structs; both shaders composed and validated by naga.
- [x] Per-room light lists in the hidden-block rank.
- [x] `EXA_SCENE_SEED` debug aid; GPU check on `arenao1` (composite against per-pixel draw).
- [x] Real-installation checks (`EXANIMA_DIR`) run with the crates' tests on the worker.
- [x] READMEs, `docs/RENDERING.md`, `docs/DEBUGGING.md`; `docs/PLAN.md` is updated by the reviewer.
- [x] fmt, clippy -D warnings and tests for exa_render and exa_world; `openspec validate --all --strict`.
