## 1. Research

- [x] 1.1 RB1 (Q11) in `exe.asm`: the stagger's pick (Get Up 2 when lean · forward ≤ 0), keys 3/2 and the clock at the key before, the blends in frames (50/40/20), the get-up start in full (threshold 0.2/−0.2/0, 100 ticks, 0.25, 0.95, 10 cm, blend 20 + whole capture), the feet-flag clearer it calls, the daze block, the standing branch (0.8, fast speed² 0.3, Rebalance wait 27, falls branch 41, decrement at the common end), the weak branch, the heading toward a fall and its call site, the re-pick (0.4), the energy while getting up; constants read from the PE; findings in `context/re/body-feet.md` §16

## 2. Implementation

- [x] 2.1 `exa_anim`: `Animator::capture_key`, `Animator::jump_in`, the sampler's capture test by key
- [x] 2.2 `exa_physics`: `Fallen` as a marker, `Drive::posture_wait`, `Ragdoll::turned`, `Balance::fast_velocity`
- [x] 2.3 `exa_combat`: `track_condition` and `dying_tick` in `CharacterStage::Posture`; `ticked_lives`; `regenerate_red` (upkeep), `keep_health` (attack), `regenerate_health` for the fighters not ticked (`ticked_by_its_character`); the daze gated by the condition; the posture switch's wait
- [x] 2.4 `exa_game::balance`: `respond`, `steady`, `pick_rise`, `waits_for_support`, `rise_heading`, `turns_to_rise`, `fall_heading`, `rising_energy`, `begin`, `take_posture`, `pick_again`, `turn_toward_fall`, `recover_rising`; `fall_off_balance` and `get_up` removed
- [x] 2.5 `exa_game`: the fall check in the posture stage; `bearing`/`rising` by branch and condition; the get-up clock's encumbrance (`Pace::encumbrance`); the later stages' run condition

## 3. Tests

- [x] 3.1 Unit (`exa_game` `balance_test.rs`): `upright_bodies_steady_themselves`, `moderate_leans_stagger`, `far_fast_lost_or_sideways_leans_fall` (float 0.8/0.4/0.3/0.1 against the double thresholds), `unsteady_bodies_steady_themselves`, `get_ups_pick_by_the_pelvis_with_hysteresis`, `lying_bodies_wait_100_ticks_for_support`, `get_ups_face_the_way_they_rise` (18.1°/18.3°), `falls_turn_the_heading`, `a_get_up_restores_energy`, `responses_start_as_the_original`, `the_posture_stage_starts_and_marks`, `rebalances_hold_the_switch_41_ticks`, `lying_still_it_gets_up_by_its_pelvis`, `a_body_that_rolls_over_changes_its_get_up`, `getting_up_energy_rises_faster`
- [x] 3.2 Unit: `a_stagger_jumps_in_and_holds_the_first_key` (`exa_anim`: 5.75 and 16 ticks, −9.25 and 27, the first tick at 0.75), `a_body_killed_in_its_posture_stage_leaves_its_tick` (`exa_game` stages), `the_bearing_reads_the_posture_stage`, `get_ups_rise_in_their_first_quarter`, `clocks_run_at_the_originals_rates` (0.7125 in plate; 0.75 on the tick the posture stage starts a get-up), `a_rebalance_holds_the_switch_off` (`exa_combat` stance); the condition, collapse, regeneration and fall tests run per ticked character
- [x] 3.3 Corpus (skipped without `EXANIMA_DIR`): `the_installed_posture_clips` (action indices 3, 0x0E, 0x0F, 0x10, 0x11, 0x21, their modes, the get-ups' key times, the condition's actions)

## 4. Verification

- [x] 4.1 In game (light runs on the RTX 4000, `--fixed-frames`, `play arena`): `--shove 30`/`60`: Defence (daze), Rebalance at 0.61 restarted each tick while dazed, down at 0.26, Fall Backward, Get Up 2 109 ticks later, Idle 155 ticks after, balance 1; `--shove 90` (a run with stray walk input before the blow): stagger into Get Up 2 at clock −9.1 (−9.25 once the get-up's first tick runs at 0.75; a later run without the stray input falls on its back and gets up as `--shove 60`, its Get Up 2 clock 0.75 then 0.9 a tick), Idle 144 ticks later; `--opponent`: six hits, collapse at E 0.555 (blend 194), Rebalance +169, dead +250; `--still-opponent --health 0.085,0.203 --press Tab@50`: full regeneration rate at yellow 0.1755 (the rule's value)

## 5. Documentation

- [x] 5.1 `docs/formats/RSQ.md` (Falls and get-ups; Condition; Motion playback), `docs/formats/OTHER.md` (Regeneration), `docs/PLAYER.md`, `docs/PLAN.md` (Phase 3, next steps), READMEs of `exa_game`, `exa_combat`, `exa_physics`, `exa_anim`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
