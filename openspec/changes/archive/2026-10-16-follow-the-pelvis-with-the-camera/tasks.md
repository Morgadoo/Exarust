## 1. Camera

- [x] 1.1 `exa_game::player`: `camera_focus` (the player body's root part drawn at the fixed overstep, else the feet); `ChaseCamera` with `ahead`, `before`, `shake`/`shake_before`, `kick`, `tick` (the shake carried on: 0.96 of its change, then 0.99; zero within 1e-11 and 1e-8 m²), `advance` (ticks once the frame passes their start, nine at most, backlog dropped) and `shown` (backward vector renormalised, zoom, shake while the present offset is above 1e-9 m²); `chase_camera` places the eye from them
- [x] 1.2 Tests: `the_camera_ticks_every_17_ms_and_at_most_nine_a_frame` (10 ms into the first tick, 7 ms ahead; a 1 s stall runs nine ticks), `the_camera_shows_its_last_two_ticks_blended` (zoom halfway, vector renormalised, shake 1 → 3 cm shows 2 at half, none once the present offset is under the floor), `a_kick_rings_the_camera_down` (a 1 cm kick shows 0.5 cm at 0.75; one tick leaves 2.8908 cm over 1 cm; it swings through zero, still moves at 240 ticks, is zero by 360; a kick under the floor is not shown; one within both stop limits is dropped by the next tick), `the_camera_follows_the_pelvis_or_the_feet_before_a_body` (the parentless part at 1 m); `zoom_is_field_of_view` kept
- [x] 1.3 Verification-run correction: the first implementation had sources set the shake outright, each tick copying it; REA showed the camera tick carries the offset on and the shake call adds to one field and takes from the other, so `kick` and the ring-down replace the public `shake` field (not exercised in game: nothing kicks the camera yet)

## 2. Verification

- [x] 2.1 REA: the frame driver (17 ms steps, 9 at most, snap, t), the placement (follow node via its body's interpolation, lerped and renormalised backward vector, zoom lerp, FOV, shake lerp added to the eye while the last tick's is above 1e-5); notes in `context/re/master-movement-19-nav-grid.md`
- [x] 2.2 Corpus: not applicable (no data read); the camera's tests run without the game
- [ ] 2.3 In game through `gpu_run.sh --gpu 4000` (light run, `play arena`): not run: the RTX 4000 had 892 MiB free against the light run's 1200 MiB (gnome-shell 15 GB, the retail game 2 GB); the rule is covered by the unit tests
- [x] 2.4 Remove the probes from the tracked tree

## 3. Documentation

- [x] 3.1 `docs/PLAYER.md` (Camera), `docs/PLAN.md` (Phase 3), `crates/exa_game/README.md`
- [x] 3.2 `openspec validate --all --strict`
- [x] 3.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
