# Tasks

- [x] Port the sight program as `MindSightMaterial` with the shipped clamp and the depth test off.
- [x] Build the sight sphere from the engine's UV sphere, shaped once.
- [x] Take the object matrix's columns from `sight_basis`'s rows.
- [x] Make the noise through `ScreenNoise::field` with both weights 0.5 on the game clock; reuse `noise_bytes`.
- [x] Return before any work without an anchor; update materials in place.
- [x] Drop the lane's force GI registration in favour of main's `thaum::effect_light`.
- [x] docs/RENDERING.md (Mind sight), crate READMEs.
- [x] fmt, clippy and tests for `exa_game` and `exa_render` on the worker.
