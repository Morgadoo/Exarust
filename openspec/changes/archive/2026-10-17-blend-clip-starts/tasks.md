## 1. The clock and the sampler

- [x] 1.1 Answer Q7 with REA and `exe.asm`: the base rate (race `0x5C`, the (80/83)/scale store overwritten), the posture block's rates (1.25, 1.5 − 0.5·encumbrance, 2), the clock block (0.6, tone > 0.5, attacks without 0.6) and the end test; constants read from `Exanima.exe`; notes in `context/re/body-misc.md` §2.9
- [x] 1.2 Re-read the start, the plain, whole and physical captures (bone test on the 1-based skin bone), the sampler's bracket, lead-in, eases (3.0, 2.0, 0.11, 8.0) and rebase, and the hold weight and its captures (0.05, 0.99, 1.19e-7)
- [x] 1.3 `exa_anim::Animator`: blend, ease, the pending capture, the capture, the last pose, the hold weight; `start`, `blended`, `capture_whole`; `play_loop`/`play_once` start with no blend, linear
- [x] 1.4 `exa_anim::clock`: `tick` (the end test, loops by the last key, the follow-up with its blend, eased smoothly), `sample` (the bracket, the lead-in from the capture, the rebase, slerp and lerp, the walks' z left out, the hold), `take_capture`, `hold_pose`, `follow_up_blend`, `in_place`, `clip_progress`
- [x] 1.5 `exa_formats::races`: the clip-rate base (`0x5C`); `exa_physics::body::RaceMuscles::clip_rate`, `gait_rates`
- [x] 1.6 `exa_game::limp`: the capture from the physical turns, `clip_frames` (walking, getting up, attacking, off balance, standing), the hold from the drive; `rising` by the original's progress; `CharacterBody::walked`
- [x] 1.7 `exa_physics::body::Drive`: `sag`, `dazed`, `hold_weight`, `holds_pose`; the sag fades 0.01 a tick, 0 once fallen
- [x] 1.8 `exa_combat::swing`: `Swing` from the weapon record, its rate; attacks start eased smoothly; `exa_game::balance`: falls with the fast ease, get-ups with a blend of 20 and a whole capture
- [x] 1.9 The body trace's `blend` and `hold`; the duellist's strike range 1.15 m

## 2. Tests

- [x] 2.1 Eases pinned (0.15625, 0.455, 0.940, 0.999); the follow-up table (0, 20, 60, 2 × recovery); only the walks and the footwork in place
- [x] 2.2 The first sample is the capture (plain: the pelvis's target, the thorax's physical turn); one get-up tick later 0.9/40 of the way
- [x] 2.3 Clips reach their first key on time: Get up 44 ticks, Get Up 2 38, Idle 33; the blend ends there and the clock loses it
- [x] 2.4 A get-up ends at its last root (6.1, 81.7, −1.8) cm and the stance blends from the last root sampled, smoothly
- [x] 2.5 Loops wrap by their last key and blend from it back to the first; a clip without a follow-up holds its last key; a whole capture takes every bone; a first clip starts from its keys; the hold's weights; progress
- [x] 2.6 The clocks pinned: 0.6, 0.75, 0.9 (0.72 at tone 0.8, none at 0.5), walking 0.764, running 0.894, backpedalling 0.736, fists 1.62, a ghoul's 0.9
- [x] 2.7 A whole capture starts a get-up from the physical pose (a thigh turned 0.4 rad off: its target follows it)
- [x] 2.8 The swing: a sword (0.3, 0.6) at 0.866 / 1.299 / 0.975 with recovery 7, fists 1.62, a stab's early ramp, the energy rule
- [x] 2.9 The hold weight (1, 0.9375, 0.25, 0) and the sag's fading
- [x] 2.10 Corpus checks (skipped without `EXANIMA_DIR`): every race's clip-rate base (1.0, the ghouls 1.5) and the human's gait clip rates

## 3. Verification

- [x] 3.1 In game through `gpu_run.sh` with traces, against a C9 build: R3 `--shove 60` (the Get Up 2 → Idle switch, the overshoot, the clocks), R4 `--opponent` (hits, attack time, joint motion at switches), a scripted walk (Idle → Walk → Idle, the loop, in place), R1 standing, R2 scripted controls, R6 `--limp`, `character "One Handed/Attack Mid"`; no validation errors from the change
- [x] 3.2 Remove probes and dumps from the tracked tree
- [ ] 3.3 R5 `play exanima01`: not run, the GPU ran out of memory loading it beside the user's retail game; to rerun when the GPU is free

## 4. Documentation

- [x] 4.1 `docs/formats/RSQ.md` (Physical body; the new Motion playback section; the motion set's mode bit), `docs/PLAN.md` (Phase 3), `crates/exa_anim/README.md`, `crates/exa_game/README.md`, `crates/exa_physics/README.md`, `crates/exa_combat/README.md`
- [x] 4.2 `openspec validate --all --strict`
- [x] 4.3 `make check` (fmt, clippy -D warnings, tests)
