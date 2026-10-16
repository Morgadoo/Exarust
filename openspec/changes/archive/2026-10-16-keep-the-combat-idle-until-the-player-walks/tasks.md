## 1. Research

- [x] 1.1 REA: the exploration steering's call graph (the clip start only through the set-stance helper; its indirect calls are line-of-sight queries), the Tab handler's (no clip start), the overlay copy in the motion constructor (each defined clip pointer, whole); recorded in `context/re/torch-arm-combat-idle.md`
- [x] 1.2 Live retail, read-only: combat mode on, Cmbt - Idle with the Defence stance kept at the exanima01 start for over an hour; a watcher logging every change of combat mode, motion and stance saw only an attack before the game was closed, so the exploration case rests on REA

## 2. The steering

- [x] 2.1 `exa_game::player`: the exploration steering starts the walk, run or backpedal from any looping clip and, standing, ends them and the interim combat walk in Idle; it starts no other clip (`explore_clip`)

## 3. Tests

- [x] 3.1 `player`: Tab off in Cmbt - Idle with the Defence stance keeps it for 200 ticks and through a quarter turn, W walks, released it stops in Idle; Tab off in the guard or in Cmbt - Idle as the stance starts Idle at once; the steering ends the gaits and the combat walk, leaves Cmbt - Idle, the guard and a clip played once, and starts a gait from any loop
- [x] 3.2 Corpus (skipped without `EXANIMA_DIR`), `torch`: "Torch" (parent One Handed) defines Walk, Cmbt - Idle, Attack Mid and Idle, the guard is One Handed's; on the `humanbase.rfc` offsets Idle's elbow 122.6°, upper arm 39.8° out and 11.4° forward, wrist (−16.4, 1.4, 18.7) cm from the shoulder; Cmbt - Idle's 101.5°, 24.9°, 31.8°, (−8.6, −3.0, 31.6) cm

## 4. Verification

- [x] 4.1 In game through `gpu_run.sh --gpu 4000`, light runs (`--no-gi --no-supersample --size 960x540`), before and after, with the joint trace (results in `context/re/torch-arm-combat-idle.md` §5):
  - exanima04 (exanima01 ran out of GPU memory even with 3 GB free): standing and walking with the torch (the clips' arm unchanged, Idle within 0.05° of the clip); Tab, a click and Tab off: Attack Up in Torch, then Cmbt - Idle, kept after Tab until W (before: Idle at Tab off); 1H & Torch in combat
  - arenao1 `--still-opponent`: the unarmed guard at 8 m and the One Handed combat idle at 2.96 m, unchanged
- [x] 4.2 Remove probes and scratch files from the tracked tree (none were added)

## 5. Documentation

- [x] 5.1 `docs/PLAYER.md` (Movement; The combat stance; The carried torch), `docs/formats/RSQ.md` (Motion set: overlays, the per-bone word), `docs/PLAN.md` (Phase 4), `crates/exa_game/README.md`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt --all -- --check`, `cargo clippy --profile play --workspace --all-targets -- -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
