# Tasks

- [x] Verify every rule of the component against the assembly: the planes' source (the camera's backward axis, not the sun), the vertical limits (record floor and ceiling, or the box's x edges), the terrain-only fitted mode, the conversions, the double constants, the fewer-than-two-corners exit.
- [x] Cut the view volume with `exa_physics::intersect::ConvexVolume`; drop the duplicated clip.
- [x] Fit each active camera in terrain levels, keep the last matrices on a failed frame, keep Bevy's cascades elsewhere and with `--bevy-sun-cascades`.
- [x] Widen the level box by the terrain's tiles.
- [x] No real-data check applies (the fit reads no game data); nothing runs without `EXANIMA_DIR`.
- [x] Update `docs/RENDERING.md`, `docs/DEBUGGING.md` and the crate READMEs; `docs/PLAN.md` is updated by the main session.
- [x] `make check` (remote: `cargo test -p exa_render -p exa_world`, `cargo clippy -p exa_render -p exa_world -p exarust --all-targets -- -D warnings`).
