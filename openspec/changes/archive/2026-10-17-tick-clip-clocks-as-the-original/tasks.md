## 1. Research

- [x] 1.1 The posture block in `exe.asm`: the off-balance branch's rate (1.25, the get-up's 1.5 − 0.5·enc) and its jump to the block's end, the weak branch (tone 0.85, no rate), the kick (2 × base, condition 0x8000), the clinch (2 × base), the combat idle (1 × base), the walk and run (none), the weapon switch, the powers and every other clip (1 × base), the end's stance reset (1 × base); constants read from `Exanima.exe` through REA; findings in `context/re/master-animation.md` (M14 findings)
- [x] 1.2 The clock block: the get-up's tone gate (0.5) and 0.6, attacks 5–12 without the 0.6, the 0x80 bit, the run blend's −0.02 and the lean's 0.95, the end test
- [x] 1.3 RB6/RB7: static only (the retail game was not running); RR1/RR7 left for a live session

## 2. Implementation

- [x] 2.1 `exa_formats::rsq::Clip::loops`
- [x] 2.2 `exa_anim::clock`: `STANDING_PACE`, `Bracket`, `bracket`, `Animator::bracket` (the progress written first), `blend_keys`, `hold_toward`, `keep`, `sample` as their composition; `tick` keeps `rate` and `ended`; `start` resets the progress
- [x] 2.3 `exa_anim`: `TICKS_PER_SECOND`; `Animator::base`, `owed`, `rate`, `progress`, `ended`; `play_clips` by the accumulator; the pose-only `sample` through the bracket; `FRAMES_PER_SECOND`, `speed` and the first-to-last `progress` removed
- [x] 2.4 `exa_combat::Condition::STILL_CLIP_MASK`, `clip_stands_still`
- [x] 2.5 `exa_game::limp`: `clip_rate`, `clip_frames` from it, `Pace::stance` and `still`, `Hold`, `clip_tick` returning it
- [x] 2.6 `exa_game::stages::sampler`: `sample_parts` and the steps (`overlay`, `femur_posture`, `clip_chain`, `ik_chain`, `twist`, `head_look`, `foot_pitch`), `SampledPose` as the parts; the carry stage reads them
- [x] 2.7 The trace's `rate`, `progress`, `clip_end`

## 3. Tests

- [x] 3.1 `exa_game` `clocks_run_at_the_originals_rates`: the Defence stance off balance 0.6 (another clip 0.75), the kick 1.2 and 0.6 weak, an attack off balance 1.25, shocked 0 (an attack 1.62), beside C10's and C11's numbers
- [x] 3.2 `exa_anim`: `the_end_test_rate_and_progress_are_exposed` (0.995, the wrap to 0.2, a follow-up and a finish), `frames_tick_by_the_body_clock` (two 50 ms frames: 8 ticks, 4.8 frames, 0.3 of a tick owed; 7.2 at base 1.5), `interpolates_between_keys` (the pose-only bracket: holds once, blends looping)
- [x] 3.3 Corpus (skipped without `EXANIMA_DIR`): `the_installed_clips_loop_and_lead_in_as_the_original` (Walk's keys 10…80 differ 19.9°, no joint turns over 1.3° across the wrap; Idle, the Defence stance and the combat idle hold their first key before it (first and last keys within 0.1°); Get up 44, Get Up 2 38 and Idle 33 blended ticks)
- [x] 3.4 `exa_game` `the_trace_has_a_row_per_tick` across a loop wrap: the clock 0.6 a tick, `rate` 0.6, `progress` clock / 30, `clip_end` once (the idle clip columns as C10 pinned them)

## 4. Verification

- [x] 4.1 Light run `--fixed-frames --trace play arena` before and after: not run, the RTX 4000 had 1030 MiB free (1200 needed; `gpu_run.sh` exit 75); the headless trace test stands in
- [x] 4.2 Probes and dumps out of the tracked tree

## 5. Documentation

- [x] 5.1 `docs/formats/RSQ.md` (Motion playback: the rate and advance tables, the sampler pipeline, bodies without a tick, the trace columns), `docs/DEBUGGING.md`, `docs/PLAN.md` (Phase 3), READMEs of `exa_anim` and `exa_game`
- [x] 5.2 `openspec validate --all --strict`
- [x] 5.3 `make check` equivalent on the `play` profile: `cargo fmt`, `cargo clippy --profile play --workspace --all-targets -D warnings`, `cargo test --profile play --workspace` with `EXANIMA_DIR`
