## 1. Stance starts

- [x] 1.1 `exa_combat::stance`: `set_stance` starts with 30 frames, smooth; the posture switch with 30, linear; `ai_stance_blend` and `take_ai_stance` (40 or 2·W19, smooth, the ease written every pass)
- [x] 1.2 `exa_ai`: the duellist's stance through `take_ai_stance` with its primary's stance blend

## 2. The daze

- [x] 2.1 `exa_combat::stance`: `daze_blend`, `take_daze` (the Defence stance every dazed tick; its start unless down or attacking, smooth); `take_stance` runs it after readiness, in `StanceTick`
- [x] 2.2 `exa_game::balance`: the fall after `StanceTick`
- [x] 2.3 `player_stance` takes the daze: Cmbt - Idle only out of it; `exa_game::player` passes `Drive::dazed`
- [x] 2.4 `exa_physics::body`: `Drive::dazed` in double precision

## 3. Attacks

- [x] 3.1 `exa_combat::attack`: `Attacker::primary_record`, `start_blend` (the other side round((20 − W19) × 0.6)); `start_attack` takes the blades and eases every start smoothly
- [x] 3.2 The swing pace's requirement names the primary striker (`pace_swings` reads it since the rebase of C10 onto L3)

## 4. Tests

- [x] 4.1 `stance`: the helper's 30 smooth, the switch's 30 linear, the duellist's 40/14/0, the daze's 45/129/300/30, the dazed fighter's start and no restart, down or attacking, the body tick's daze and blend back, the dazed player's guard
- [x] 4.2 `attack`: the other side's 12/8/12/0 and the main side's 0 (the primary claw's swing is pinned by the rebase's `swing` test)
- [x] 4.3 `body`: the daze's threshold at 0.95f and the next float up
- [x] 4.4 Corpus (skipped without `EXANIMA_DIR`): the workspace's real-data tests still pass

## 5. Verification

- [x] 5.1 Light runs through `gpu_run.sh --gpu 4000` (`--no-gi --no-supersample --size 960x540`), no panic or validation error:
  - `--opponent --trace play arenao1`: six sword blows kill the player; after each it guards (Defence stance, blend 106–150) and, energy back, the posture switch blends to Cmbt - Idle (30 frames), the targets turning 0.2–0.4° a tick (10.8–17.6° in one tick on the C10-only build); the first Defence → Cmbt - Idle switch turns a hand's target 17.4° once (25.5° before), the capture of its physical turn
  - `--shove 60 --trace play arena`: dazed to 0.13, the Defence stance starts (blend 276), Fall Backward replaces it, Unconcious 1, Get Up 2, Idle with no jump; the capsule moves at most 2 cm a tick
  - `play exanima01`: not run to the end, out of GPU memory (the desktop holds 18 GB of the RTX 4000)
- [x] 5.2 Remove probes and scratch files from the tracked tree (none were added)

## 6. Documentation

- [x] 6.1 `docs/PLAYER.md` (Combat mode, The combat stance), `docs/formats/RSQ.md` (Motion playback: Swing, Stance starts), `docs/PLAN.md` (Phase 4), `crates/exa_combat/README.md`
- [x] 6.2 `openspec validate --all --strict`
- [x] 6.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
