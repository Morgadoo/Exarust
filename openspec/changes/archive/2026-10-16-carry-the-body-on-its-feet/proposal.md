## Why

A standing body in Exarust is held up by its own invention: gravity scaled
by (1 − tone × energy) on every part, so a standing body weighs nothing,
and a controller hold that pulls its pelvis to where the capsule carries
the pose's pelvis, which the capsule also carries up and down. The
original has no capsule: each limb particle falls by g(1 − a) + carry·a,
and a pelvis-height servo (the lift) turns the carry into an upward pull
that holds the pelvis at a target height over the ground under the feet.
C7 left the vertical hold as a stopgap; its damping is critical for the
pelvis alone, so the end of a get-up overshoots 10.7 cm and rings for
about 90 ticks.

Phase 3 (Bodies), milestone M3 "standing man". This is change C8 of the
body-controller plan; it depends on C6 and C7 (done). Q1 is answered
(height comes only from the lift and its target), so it no longer blocks.

## What Changes

- The carry: after inertia each particle of a living part over half its
  tone t moves by g(1 − a) + carry·a, any other by g. a = t × the part's
  support share (pelvis and thighs 1, lower legs 0.5, feet 0.25) or t × E
  for the upper body. The character tick sets the carry to gravity, then
  takes the lift off it.
- The lift, with support only: standing f = 0.075(h + 15), at most 1.5
  (negative: no lift; crouched past 0.9 down to −10); moving
  f = (0.4·run + 0.2)(h + 5), at most 2 (negative: none); h = target −
  pelvis height (cm). Damped: f −= (1 − |f|/(|f| + 2))·E²·v_y. Over a
  tone of a half f ×= 2(t − 0.5) and carry = g(1 − f·S).
- The target pelvis height: per leg, ground + crouch·60 + (1 − crouch)·
  (the clip root's height over the animated ankle + the ground offset).
  The ground offset is the sole height standing; moving, the ankle's height
  over the lowest ankle of the walk's first pose, plus the sole (it also
  feeds the grounded test). The target is the lowest foot's, blended by
  half toward the other foot's under the original's spread test; standing
  it is capped at ground + sole + leg length and raised 5 cm. No ground:
  −1000 m, no lift.
- The capsule carries a standing body only across the ground and about
  the vertical: its rise is dropped from the carry.
- Removed: the `(1 − tone·energy)` gravity; the vertical part of the
  controller hold. Kept, Exarust's own until the support pins (C14): the
  hold across the ground (0.168 × tone² × E⁴, critically damped).
- The body trace writes the lift's target in `target_pelvis_height`.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 with REA and `exe.asm`: the
  lift servo, the feet routine's target-height block, the limb step's
  per-particle carry, the carry's reset and propagation, the muscle
  setup's support shares (the lower leg's is 0.5, which corrects the
  notes), the living sampler's ankle point and ground offset, the gait
  routine's lowest ankle; constants read with `read_bytes`
  (`context/re/body-drive.md` §4–5 and the new §14).
- Exarust runs through `gpu_run.sh` with `--trace`: `play arena` standing,
  `--autowalk`, `--shove 60`, `--opponent`, `play exanima01` (with its
  stairs), `--limp character`, `character`.

Out of scope:
- the frame anchor (C9), the support pins and the horizontal hold's
  removal (C14), locomotion by the carry and the capsule's removal (C18);
- foot contacts and planting (C12), leg IK (C13), the walking feet: swing
  flags, the probe ahead and the gait-smoothed ground (C19); so stairs
  regress until then;
- the crouch (Exarust has none), the stepping and resting variants of the
  lift, the collapsing tone², the large-body shares, jogging and sprinting
  gait floors; the original's falls, get-ups and clip blends (C10, C11).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: the carry, the lift and the target pelvis height; the
  controller carrying across the ground and holding horizontally only;
  the muscles without the tone gravity; support's ground offset; the
  trace's target column; the get-up without a hop.

## Impact

- `exa_physics` (`body.rs`: `integrate`, `carry`, `aim`/`hold`,
  `hold_on_controller`, `Drive::carry`/`lift_target`/`lift`; new
  `body_lift.rs` and `body_lift_test.rs`; `body_balance.rs`: `Leg::offset`;
  `body_muscle.rs`: `Bearing::gait_floor`; tests in `body_drive_test.rs`,
  `body_floor_test.rs`, `body_contact_test.rs`)
- `exa_game` (`limp.rs`: the walk's lowest ankle; `trace.rs`: the target
  column)
- `docs/formats/RSQ.md` (Physical body), `docs/PLAYER.md`, `docs/PLAN.md`,
  `crates/exa_physics/README.md`, `crates/exa_game/README.md`
