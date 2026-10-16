## Why

Exarust's pose drive is its own invention: a critically damped spring per
particle toward the animated **world position** of each part, on the
pelvis (twice as strong, standing in for the support forces), the spine,
the head and the feet, with a fallen body's targets re-measured from its
own pelvis, its bulk motion damped and its net shift and turn taken back
out, and its tone dropped to 0.5. The original's pose drive only turns
parts: each particle's target is its rest vertex under the part's animated
rotation, placed at where the part's origin already is, so it has no
position or height target; it damps the particle's own velocity along the
offset, one-sided, and it is scaled by the support S² that C6 now
measures. A fall keeps the tone, and with S = 0 a fallen body is left to
its link muscles.

Phase 3 (Bodies), milestone M3 "standing man". This is change C7 of the
body-controller plan; it depends on C5 and C6 (done).

## What Changes

- The pose drive acts on the pelvis, lumbar, thorax and feet (gain
  0.03·K, K = 19200/83² = 2.787) and the head (0.01·K); thighs, shins,
  arms and hands have none (as since C5).
- Each particle's target is R·r + O: its rest vertex r under the part's
  animated rotation R (the body root's turn times the clip's), placed at
  the part's origin O measured on its particles after inertia and the
  carry. No position or height target.
- The push is G·o·(1 − c), c = o·v/(s|o|²), only when c < 1, its vertical
  cut to 0.1; o = target − particle, v the particle's displacement over the
  last tick (before this tick's inertia), s the stance factor.
- G = gain·t²·T·E⁴·(0.5T + 0.5)·S², t the tone, T = tone·s, E the energy,
  S the support; only while the body has support (the has-support flag).
  Idle 0.0314 (head 0.0105), walking 0.0497, running 0.0836.
- A fall keeps the tone; once the body lies, S = 0 and only its link
  muscles act. Energy recovers 0.005 a tick up to the most energy (1 for a
  human).
- Removed: the per-particle critical-damping PD toward world positions,
  `DRIVE`, the pelvis's ×2 `SUPPORT`, `FALLEN_TONE`, the fallen body's
  pelvis-aligned targets, its bulk damping and the removal of its net shift
  and turn, the pose's own motion (last tick's targets).
- Exarust's own, until the body carries itself on its feet (the lift
  servo, C8; the frame anchor and support pins, C9 and C14): while a body
  stands on its controller, its pelvis's origin is held where the
  controller carries the pose's pelvis, by a critically damped spring of
  0.168 a tick × tone² × E⁴ (the old pelvis gain), the same push on each of
  its particles so it never turns the pelvis. Without it a body pushed
  1 cm up by the floor floats 30 cm up (gravity is compensated by tone)
  and a get-up never rises.
- Exarust's own: the capsule carries the body, so v is the particle's
  velocity in the carried frame (the capsule's motion is not velocity);
  the 25 m/s guard stays.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5: the limb step's origin write
  and pose-drive loop, the gain setup of the midline chain, the feet and
  the head, the energy cap in the body tick, the constants 0.03, 0.01,
  0.5, 1.0, 0.1 and 1.19e-7 read with REA (`context/re/body-drive.md`
  §0–2, §8–9 and the new §13; `body-controller-plan.md` C7 and the
  answers to Q1–Q3).
- Exarust runs through `gpu_run.sh` with `--trace`: `play arena` standing
  and walking, `--shove 60`, `--opponent`, `play exanima01`,
  `--limp character`.

Out of scope:
- the lift servo and the target pelvis height, the capsule carrying only
  x, z and yaw (C8); the frame anchor (C9); the support pins (C14);
  locomotion by the carry (C18);
- the midline lean on the spine's rotation, the unconscious family's
  vertical-only push, an aged character's lower head gain;
- the original's falls and get-ups (C11); the ×0.95 retention of resting
  dormant statues and golems.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: the pose drive's law, targets and gains, the
  controller's hold of a standing body, falls keeping their tone, the
  fallen body's muscles, the energy cap, ragdoll stability without
  pelvis-aligned targets.

## Impact

- `exa_physics` (`body.rs`: `drive_parts`, `pose_push`, `Drive::pose_gain`
  and `pose_strength`, `hold_on_controller`, `Part` keeps each particle's
  last displacement and its own origin's weights, `Drive` loses `before`,
  `aim`/`hold`/`fall`/`stand`, `Bulk` removed; tests in the new
  `body_drive_test.rs`, `body_muscle_test.rs`, `body_contact_test.rs`)
- `docs/formats/RSQ.md` (Physical body), `docs/PLAYER.md`, `docs/PLAN.md`,
  `crates/exa_physics/README.md`, `crates/exa_game/README.md`
