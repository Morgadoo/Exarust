# Tasks

- [x] Frame tick models: cap, clocks, idle wait, timers, deferred ring, status line, overlay, log levels (`exa_game::frame`).
- [x] Window models: display modes, refresh rate, fitting, minimum, full-screen rule, present mode, toggle (`exa_game::window`).
- [x] Apply the full-screen rule in `hw_config`; feed the cap from `FPSLimit` and `--max-fps`.
- [x] Frame loop: cap, focus release, Alt+Enter, low-power background update (`exarust::frame_loop`).
- [x] Scene pass order and list orders (`exa_render::passes`).
- [x] View rules: levels, jitters, render scale, projection, point projection, rectangle clip, stacks, copy (`exa_render::view`).
- [x] No corpus check applies (no data format is read); the settings' rules are covered by synthetic settings files, skipped nowhere.
- [x] Update `docs/PERFORMANCE.md`, `docs/RENDERING.md`, `docs/UI.md` and the crate READMEs; `docs/PLAN.md` and the component map are the integrator's.
- [x] `make check` for the touched crates (remote tests and clippy).
