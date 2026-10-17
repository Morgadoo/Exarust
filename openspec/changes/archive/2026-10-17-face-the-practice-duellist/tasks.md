## 1. The duellist's place

- [x] 1.1 `exa_world::start::duel_opponent(level, player)`: the arena ring's second slot, else 2.5 m along the player's facing, facing back at it (the distance a named constant, commented as Exarust's choice)
- [x] 1.2 Play mode spawns the duellist's root there and turns its body to the start's yaw (the ring slots face +Z as before)

## 2. Tests

- [x] 2.1 `a_duel_starts_face_to_face`: in `arena` the duellist stands at (0, 0, 2.5), 2.5 m along the facing of the player's start, facing the player; the same for a player moved (as `--at` does) or turned; `arenao3` keeps its ring slot
- [x] 2.2 The test fails on the previous placement (2.5 m along +Z, behind a player facing −Z)

## 3. Verification

- [x] 3.1 Before, through `gpu_run.sh --gpu 4000` with light settings: `--opponent play arena` for 40 s, no hit (the duellist attacks every 2.2 s from 1.08 m); a temporary probe of the blade's nearest gap to the player's part spheres: 15–21 cm short at its closest (clip progress 0.71–0.76, tip 8.5–9.8 m/s), the player facing away (its facing · the way to the duellist −1.00), no target, in its guard; `--opponent play arenao1`: six hits of six on the head, 22–30 cm along the blade; `--at 0,0,-4.5 play arenao1` (duellist behind): no hit; `--cursor 480,470 play arena` (the player turned to face it): six hits of six, down after 15 s
- [x] 3.2 After: `--opponent play arena`: the player targets the duellist at 2.46 m from the start; six blows of six land on the head (23–28 cm along the blade, 4.4–5.8 cm/tick), the first at tick 303 (about 2.7 s after the fight starts), the player down at tick 1217; the side view `--camera 5.5,1.3,4.5,90,-6` frames the duel
- [x] 3.3 Remove the probe from the tracked tree

## 4. Documentation

- [x] 4.1 `docs/DEBUGGING.md` (where the duellist stands, the side view's camera, the first blow's time), `docs/PLAN.md` (Phase 4)
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR` (545 passed, 3 ignored)
