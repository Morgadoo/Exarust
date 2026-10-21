# Tasks

- [x] Check every rule of the unit against the listing: the align wrap, pitch floor, held pitch, orbit, free zoom, frame clock, placement, preset, kick, cut-away driver and probe, plane and mesh picks, kinesis pick and the scatter.
- [x] Correct the camera rules in `nav_camera` and the live chase camera's use of them (align reference, held pitch, orbit rate).
- [x] Correct the picks in `nav_pick` and the cursor's stand-in node.
- [x] Correct the cut-away probe in `scene_nodes` (scene-word bits, drawn sectors, sphere test, overhead probe, carry).
- [x] Restore the scatter's cosine as x with the engine's sine and cosine.
- [x] Tests with hand-worked values for each corrected rule.
- [x] Real-installation check: the worker's test run reads `EXANIMA_DIR` (body, floor and level tests pass); no corpus command reads camera or pick values, skipped without `EXANIMA_DIR`.
- [x] Update `docs/PLAYER.md` and the crate READMEs; `docs/PLAN.md` is updated by the main session.
- [x] `make check` equivalent: `cargo fmt --all`, clippy `-D warnings` and tests for `exa_game`, `exa_world`, `exa_physics`.
- [x] `openspec validate --all --strict`
