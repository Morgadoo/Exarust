## 1. Changes

- [x] 1.1 `Ragdoll::park`/`unpark`/`parked`/`park_unseen` and the parked tick in `step_bodies`; test `a_parked_body_holds_until_unparked`
- [x] 1.2 `InView`, bridged with the bodies and the character tick
- [x] 1.3 `exa_game::park`: the frame's note, the manager frame's wake before the rise, `EXA_NO_PARK`; tests `the_body_tick_parks_an_undrawn_sleeper`, `the_manager_frame_wakes_the_drawn`
- [x] 1.4 The posture stage parks; the golem wake unparks

## 2. Checks

- [x] 2.1 fmt, clippy `-D warnings` and tests of exa_physics and exa_game, the workspace tests
- [x] 2.2 Grid against r192: exanima06 statues parked out of view, the gate's golems unparked; lockstep traces and the determinism grid
