## 1. Changes

- [x] 1.1 Grip reset in the balance routine's feet centre; test `the_balance_pass_puts_each_grip_back_at_full`; the quarter-turn, knee-on-a-step and dazed-sag tests adjusted to it
- [x] 1.2 The balance floor clears the step without its end; test `a_step_under_the_balance_floor_is_cleared_not_ended`
- [x] 1.3 `Locomotor::face` and `Locomotor::placed` write the facing; run by `limp::place_on_ground` and `exa_ai::drive::add_locomotion`; tests `a_placement_resets_the_twist_and_takes_the_matrix_heading`, `a_ground_placement_faces_without_the_reset`
- [x] 1.4 The whole capture keeps the thighs' targets; test `a_whole_capture_keeps_the_thighs_targets`
- [x] 1.5 The stab drive's guide normalised approximately

## 2. Checks

- [x] 2.1 exanima01, plain duel and sword duel lockstep, 2 runs each, against r169 and a build without the grip reset (context/artifacts/2026-10-30-kernel3)
- [x] 2.2 exa_physics and exa_combat READMEs; docs/re/motiledyn-map.md (block table for #18); docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.3 fmt, clippy -D warnings and tests of the touched crates on the worker
