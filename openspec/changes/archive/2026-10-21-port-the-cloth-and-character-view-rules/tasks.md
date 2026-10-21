# Tasks

- [x] Cloth driver: per-node mesh copy, skipped nodes, importer gate (`exa_world::cloth`, `exa_formats::rfc`).
- [x] Keyframe curves and slerp corrections (`exa_formats::key_track`, `exa_anim`).
- [x] Fast sine and cosine pair (`exa_assets::polyhedra`).
- [x] Abstract pathfinder and install state (`exa_world::nav::finder`).
- [x] Decomposition unit map and layer cap, centred slider, tone blend target (`exa_assets`).
- [x] Character-view tick against the original's instructions (`exa_game::char_view`).
- [x] Player-control creation state documented as a traced constructor only.
- [x] `docs/RENDERING.md`, `docs/NAVIGATION.md`, `docs/formats/RFC.md` and the crate READMEs. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are updated by the integrator.
- [x] fmt, clippy -D warnings and tests for the touched crates on the worker (`nav::tests::shipped_levels` already fails on the integration branch).
- [x] `openspec validate --all --strict`.
