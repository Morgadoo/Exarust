## 1. Changes

- [x] 1.1 `engine_math::normalize_quat`; test `the_quaternion_normalisation_is_the_four_lane_one`
- [x] 1.2 Twist, midline half turn, half lean, hunch and femur posture normalise approximately; tests loosened to the 12 bits
- [x] 1.3 Fall and get-up headings normalised approximately, their yaw the engine's heading; tests `get_ups_face_the_way_they_rise`, `falls_turn_the_heading`
- [x] 1.4 Dying heading and daze pose as read; `exa_anim` depends on `exa_physics`
- [x] 1.5 Keyless and resumed walk starts keep the jump-in key; test `a_keyless_start_keeps_the_playing_clips_jump_in_key`
- [x] 1.6 Whole capture half-leans; test `a_capture_half_leans_the_lumbar_and_thorax`

## 2. Checks

- [x] 2.1 exanima01, plain duel and sword duel lockstep, 2 runs each, against r156 (context/artifacts/2026-10-30-kernel2)
- [x] 2.2 exa_physics, exa_anim, exa_game READMEs; docs/re/motiledyn-map.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.3 fmt, clippy -D warnings and tests of the touched crates on the worker
