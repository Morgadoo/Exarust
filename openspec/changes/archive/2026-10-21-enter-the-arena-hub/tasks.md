# Tasks

- [x] Read the hub entry, session hook and click hook; record them in `context/re/arena-hub.md`.
- [x] Add `exa_game::arena_hub` (music draw, click hook, session hook) with tests.
- [x] Add `exa_audio::LevelMusicStart` and draw the hub's music in play mode.
- [x] No corpus field changes (the track names were checked against `Sound.rpk` by hand; skipped without `EXANIMA_DIR`).
- [x] Update the `exarust` README and `docs/formats/OTHER.md`. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are the integrator's.
- [x] `make check` (remote: `cargo test -p exa_game -p exa_audio -p exarust`, clippy `-D warnings`, `cargo fmt`).
