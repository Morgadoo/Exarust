## 1. Research

- [x] 1.1 In `Exanima.exe` and the decompiles: the limb link iteration's foot contact step (refresh, gate on tone 0.5, support and 10 cm under the pelvis, the touching/anchored/reached branches, approach 0.95/0.05, grip +0.001 to 1, ramp +0.01/+0.001/+0.0001 to 0.25, the 0x40/0x80 states, the walking follow 1 − 4·ramp, servo 0.3·run + 0.2 and rates 0.2/0.1/0.4, the lifted branch) and its sole step (0.5/0.1 cm × S, the orientation drive); the servo (q < 0.05) and PD (0.1/0.004/0.03, 1 and 9 cm, 0.125) helpers; the feet routine (friction 0/2.0/0 copied to the foot, the probe by the condition, smoothing, anchor ≤ ankle + 100, foot state 0x200, the sole query and floor, the swing block, no ground 0x30, grounded, level 0.9/0.1, extension); the footwork (lying, bad, unsupported, landing pass, stall, step start and re-target); the standing sampler's plant; the clearer's five callers; the muscle setup's toe (0, −sole, 20·scale) and foot friction 0.5; constants from the PE; findings in `context/re/body-feet.md` §18

## 2. Implementation

- [x] 2.1 `exa_physics`: `FootState`, the foot contact fields of `Leg`, built with the body (`body_feet.rs`)
- [x] 2.2 `exa_physics`: the feet's states before they are measured (`feet::plant`, the footwork's landing pass and step)
- [x] 2.3 `exa_physics`: the feet's measurement in `measure_balance` (friction, probe, smoothed ground, anchor height, floor, no ground, level, character ground) and the legs' extension in the lift
- [x] 2.4 `exa_physics`: the contact drive and the sole push in each iteration; the carry moves the anchors; `clear_feet`, `ankle`, `drawn_foot`, `Balance::level`/`extension`/`landings`; `Bearing::walking`/`off_feet`/`progress`
- [x] 2.5 `exa_game`: the bearing from the condition and the clip's progress; the get-up's clear; the trace's foot and ankle columns; the overlay's anchors and the readout's feet

## 3. Tests

- [x] 3.1 Unit (`exa_physics` `body_feet_test.rs`): `legs_take_their_friction_from_their_feet`, `the_servo_closes_its_rate_of_the_gap` (0.1 and 0.08 cm), `the_grip_holds_and_lets_go_over_the_anchor` (−0.3, 0.1; 0.005/0.0375; ½ at 5 cm, none at 9.5), `the_grip_rises_over_1000_iterations`, `a_step_reaches_its_anchor_within_5_cm` (5.1 / 4.9 cm), `a_lifted_foot_moves_only_up`, `the_drive_needs_tone_support_and_a_foot_under_the_pelvis`, `the_toe_is_held_over_its_floor` (0.5, 0.3, 0.2, 0.1 cm), `feet_are_planted_lifted_or_stood_on_their_stance_spots`, `anchors_sit_on_the_smoothed_ground`, `the_stance_levels_and_the_legs_extend` (0.975, 0.25), `a_standing_body_plants_its_feet`, `a_quarter_turn_in_place_keeps_planted_feet_within_a_cm` (0.64 cm, six steps; 10 cm without the drive), `standing_feet_grip_through_a_light_shove` (0.1 cm, chest 4 cm), `feet_on_a_step_anchor_on_their_own_ground` (24.5 / 9.5 cm, level 0.60)
- [x] 3.2 Unit: the 5 cm standing probe in `down_the_query_starts_at_the_ankle_5_cm_wide`; the tests' standing body on its anchors (sole 0, legs 3.3 cm longer to the lift); the climb without the stopgap now sinks instead of toppling; `the_bearing_reads_the_posture_stage` (walking, off its feet); the trace's empty foot and ankle columns for a legless body
- [x] 3.3 Corpus: no format change; `cargo test --profile play --workspace` with `EXANIMA_DIR` (the races' pin factor 1.25 the grip reads is pinned by `character_stats_test`)

## 4. Verification

- [x] 4.1 In game (light runs on the RTX 4000, `--fixed-frames`, `play arena`): standing, both feet 0x2 on anchors 9.7 cm up, ankles 0.09–0.11 cm off (0.37–0.44 cm after the stop's steps), 0.004 cm of drift over 200 ticks; standing quarter turns (A, D) carry the planted feet (0.07–0.09 cm off their anchors); walking, no foot touches; stopping, the feet step onto their spots in 45 ticks, balance 1; `--shove 60`: Rebalance at 0.64, down, Get Up 2, Idle, the feet stepping back; `--opponent`: five hits, collapse, death; exanima01's stairs did not load (out of GPU memory) — the step unit test stands in

## 5. Documentation

- [x] 5.1 `docs/formats/RSQ.md` (Feet; friction, support, the carry, get-ups, the trace), `docs/PLAN.md` (Phase 3, next steps), `docs/PLAYER.md`, `docs/DEBUGGING.md` (overlay, readout, trace), READMEs of `exa_physics` and `exa_game`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
