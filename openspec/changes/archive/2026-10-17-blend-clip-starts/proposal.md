## Why

A living body switches clips in one tick: `play_once` jumps to the next
clip's first key, every clip loses its root's travel, and the clock runs at
60 frames a second for everything. The end of `Get Up 2` moves the pelvis
target 36 cm in one tick, an attack's end throws the arm 70–85° off its
target, every walk cycle pops 20° at its loop, and the original's clocks
(0.6 frames a tick standing, 0.9 getting up, about 1.1 attacking) are not
used. The original starts every clip from a capture of where the body is
and blends to the clip's first key, keeps the root's travel of every clip
but the walks, and runs each state's clock at its own rate.

Phase 3 (Bodies), milestone M3 "standing man". This is change C10 of the
body-controller plan; it depends on C1 and C9 (done) and answers Q7 (the
clip clocks).

## What Changes

- Clip clocks: frames a tick by state, from the race's clip-rate base
  (`races.rdb` `0x5C`, decoded here): 0.6 × base standing, 0.75 off
  balance, tone × 1.5 × base × 0.6 getting up (above half tone), the
  weapon's swing rate attacking, the gait's clip rate × (0.8 × speed + 1)
  walking. Loops wrap by their last key, blending from it back to the
  first. Progress is (clock − blend) / the last key.
- Clip starts: a start captures the root, the pelvis's and legs' last
  targets and the upper body's physical turns (a get-up from the floor: the
  whole lying body, the pelvis on the frame); the clip blends from the
  capture to its first key over (first key + blend) frames, linear, smooth
  or fast; aiming past the first key ends the blend (the clock loses it).
- Follow-ups: a clip played once ends into its follow-up with a blend of
  0 below action 0x1F, 20 from 0x1F, 60 from 0x21, twice the weapon's
  recovery after an attack, eased smoothly.
- Root travel: kept for every clip but the walk, run and backpedal (and
  Exarust's interim `Combat Walk`), whose z the body's motion replaces.
- Pose hold: the sample is pulled toward the pose the body holds (its
  pelvis's and legs' targets, the other parts' physical turns) by the
  original's weight, 1 − (1 − S)²(1 − s)(E/Emax)⁴ dazed, else (1 − S)s.
- Swing pace: an attack's clock runs at its weapon's swing rate; attacks
  start eased smoothly; falls start with the fast ease; a get-up with a
  blend of 20 and a whole capture.
- The duellist's strike range (Exarust's own) is 1.15 m: link-muscle arms
  lag more at the original's swing speed.
- The body trace adds `blend` and `hold`.

Evidence:
- REA (Ghidra 12.1.4) decompiles of the original's captures, `exe.asm` of
  its character init, clip start, living sampler (bracket, lead-in, eases,
  rebase, hold weight), body-tick clock block and end test, daze block and
  attack routine, with every constant read from `Exanima.exe`
  (`context/re/body-misc.md` §2.9).
- The installed `humanoid.rsq` (key times and roots) and `races.rdb`
  (clip-rate base, gait clip rates).
- Exarust runs through `gpu_run.sh` with `--trace` and `--joint-trace`:
  `--shove 60 play arena`, `--opponent play arena`, a scripted walk,
  `play arena` standing, `character "One Handed/Attack Mid"`.

Out of scope:
- the original's falls, staggers, Defence and Rebalance starts and the
  get-up re-pick (C11), and the stagger jump-in;
- the locomotion clock's stride modulation, its start key and the run
  overlay (C18);
- the daze's Defence stance and its blend round((1.05 − E)·300), which
  need the combat stance; the blow sag's writer, the swing's alignment
  wait, skill, recoil and slow state (C21);
- the torso twist in the captures, the femur posture, the foot contacts'
  flag in the capture, the armour encumbrance, the creature classes' own
  clock bases.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `motion-playback`: clip clocks, loops by the last key, starts from a
  capture, follow-up blends, root travel kept but for the walks.
- `physical-body`: clips sampled every tick at the original's clocks; the
  get-up's start from the lying body; the pose hold; the trace's blend and
  hold columns.
- `combat`: the swing's pace and the attack's end blend.

## Impact

- `exa_anim` (`lib.rs`: `Animator`, `ClipPose::at`, frame playback; new
  `clock.rs`, `clock_test.rs`)
- `exa_game` (`limp.rs`: the clip tick, `clip_frames`, `physical_turns`;
  `balance.rs`: the fall's and get-up's starts; `trace.rs`)
- `exa_physics` (`body.rs`: `Drive::sag`, `dazed`, `hold_weight`,
  `holds_pose`; `body_muscle.rs`: `RaceMuscles::clip_rate`, `gait_rates`;
  `lib.rs`: `CharacterBody::walked`)
- `exa_combat` (new `swing.rs`; `start_attack`), `exa_ai` (strike range),
  `exa_formats` (`races.rs`: the clip-rate base), `exarust` (`viewer.rs`)
- `docs/formats/RSQ.md` (Physical body, Motion playback), `docs/PLAN.md`,
  `crates/exa_anim/README.md`, `crates/exa_game/README.md`,
  `crates/exa_physics/README.md`, `crates/exa_combat/README.md`
