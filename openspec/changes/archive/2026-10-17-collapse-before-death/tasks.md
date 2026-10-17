## 1. The collapse

- [x] 1.1 `exa_combat::collapse`: `collapse_blend` (round((1.2 − E) × 300) in double, ties to even), `jitter_heading` ((draw − 0.5) × 0.5 per axis, renormalised), `lose_tone` (−0.003 in double, death at ≤ 0.25 with tone 0.01), `fade_pins` (−0.0003 in double, 0.01 once under 0), `rebalances` (size < 1.2 and balance < 0.8, double), `throw_arms_wide` (Rebalance looping, 20 frames, linear, not restarted while it plays), the `Collapse` component
- [x] 1.2 `start_collapse`: condition collapsing, the waking flag cleared, Fall Backward looping with the blend (nothing restarted when it plays; held, `then` dropped), eased smoothly, the body off its controller (`Ragdoll::fall`)
- [x] 1.3 `dying_tick` (`CollapsePlugin`, in `PostureTick` after `track_condition`): heading drift by two draws of the original's generator, Rebalance once (20 frames, linear), tone and pins' gain, death (condition dead, `Ragdoll::go_limp` that tick, `GoLimp { velocity: None }`, `Collapse` removed)
- [x] 1.4 `react_to_hits`: a killing blow starts the collapse on a living body with `daze_energy(E, daze)` (removing `Fallen` and the attack), else dead and limp at once; no invented velocity

## 2. The collapsing body

- [x] 2.1 `exa_physics::body::Bearing::collapsing`: the frame follows the pelvis with no reach (`frame::follow`), the lift × tone² (`lift`), the arms 0.2 (`arm_stiffness`), the feet probed standing (`measure_balance`)
- [x] 2.2 `Drive::pin_gain` (start 0.08f), `recover_pins` (+0.001 under the double 0.08 unless collapsing, in `step`), `daze_energy` (shared with `Ragdoll::shove`), `GoLimp::velocity: Option<Vec3>`
- [x] 2.3 `exa_game`: the bearing's `collapsing` from the condition; `fall_off_balance` and `get_up` skip the dying; `go_limp` takes the muscles only when a death has not, and sets a velocity only when given; `viewer`'s `--limp` keeps its velocity
- [x] 2.4 `exa_game::sim_control`: `--pause-after-collapse <n>` (`SimControl::pause_after_collapse`, armed in `PostUpdate` after the combat systems by the first body whose condition becomes collapsing), parsed in `exarust`

## 3. Tests

- [x] 3.1 Unit (`collapse_test.rs`): `fall_backward_blends_by_the_energy_left` (60, 90, 174, 210, 345), `death_comes_at_a_tone_of_a_quarter` (250 ticks, 0.25300 on the 249th), `the_pins_fade` (0.081 living, 0.0063 after 249 ticks, 0.01 reset), `small_bodies_rebalance_under_0_8` (0.8f and 1.2f do not), `rebalance_starts_once_with_20_frames` (looping, blend 20, linear, its clock kept on the next tick), `the_heading_wanders` ((0.2425, 0.9701), (−0.3162, 0.9487), unit for 250 ticks), `a_lethal_wound_starts_the_collapse`, `the_body_dies_250_ticks_after_the_blow` (tone 0.25300, pins 0.0053, dead on the 250th, `GoLimp` without velocity)
- [x] 3.2 Unit: `a_killing_wound_starts_the_collapse` (blend from the dazed energy, `Fallen` and the attack gone, off the controller, a second cut changes nothing), `a_killing_wound_without_muscles_kills_at_once`; `exa_physics`: `collapsing_arms_are_0_2`, `a_collapsing_frame_follows_the_pelvis_exactly`, `a_collapsing_lift_fades_by_the_tone_squared` (0.648, 0.28125, 0); `exa_game`: `clocks_run_at_the_originals_rates` (collapsing 0.6 a tick)
- [x] 3.3 Unit (`sim_control`): `pause_a_second_after_a_collapse` (a living fighter arms nothing; paused 83 ticks after the tick of the first collapse; a second collapse does not move it)
- [x] 3.4 Corpus (skipped without `EXANIMA_DIR`): Fall Backward and Rebalance are looping held poses of the installed motion set (`balance::the_condition_reads_the_installed_actions` extended)

## 4. Verification

- [x] 4.1 REA and `exe.asm`: the collapse start, the dying tick (constants in double/float as read), the collapse branch of the body tick, the lift's × t² for `& 0x800`, death, the clip start's no-op, the pins' gain's start and recovery; findings and corrections in `context/re/master-body.md` §21
- [x] 4.2 In game (light runs on the RTX 4000): `--opponent --trace --joint-trace play arena`: the duellist's 7th blow (tick 1402) leaves E 0.668, Fall Backward with 160 frames of blend, condition 0x800; the body sinks with balance 1 (its chest over its feet) until its feet lose the ground at tick 1533, Rebalance (130 ticks, 1.57 s after the blow), dead on the 250th tick (1652), limp, left upper arm 59° out at death; `--shove 60 play arena` falls and gets up as before
- [x] 4.3 In game, the player killed (`--opponent --health 0.05,0.05 --camera -0.25,2.4,7.0,0,-42 --pause-after-collapse n`, light, eight runs): E 0.61–0.63, blend 171–178; at 33, 83 and 100 ticks a squat in Fall Backward's lead-in (knees 86° at 83), the chest over the feet; Rebalance at 133–158 ticks (balance 0, tone 0.53–0.60), sitting at 190, on the back at about 210; dead at 250; at 600 on its back and side, upper arms 68°/72°, one thigh 108°, the other 20°. Against the user's shots 3 and 7 and the live 0.4 s/1.2 s frames (a heavier daze, E 0.36): the corpse matches; the fall comes late (live Rebalance at like energies 0.88 s and 1.42 s), most likely for want of the stagger steps (C15/C16)

## 5. Documentation

- [x] 5.1 `docs/formats/RSQ.md` (Condition: the collapse), `docs/UI.md` (the ring through the collapse), `docs/PLAN.md` (Phase 4), `docs/DEBUGGING.md` (`--pause-after-collapse`), crate READMEs (`exa_combat`, `exa_physics`, `exa_game`)
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
