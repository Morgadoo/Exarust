## Why

PLAN Phase 3 (TTetraRBD gap). The rigid body's two-step point transform was
ported but had no caller: the locomotion routine and the pelvis-height lift
servo read the root's last-tick displacement as a running sum of per-particle
differences against the previous particles, where the original calls the
two-step point on the root's own origin.

## Evidence

REA (Exanima.exe):
- The two-step point (TTetraRBD): the weighted point over the third particle
  set into the output, the weighted point over the current set into a local,
  then output = local − output (the vector subtract reverses its operands:
  SUBPS of the first argument from the second). The weighted point helper
  multiplies each particle by its weight and adds them as
  (w0p0 + w1p1) + (w2p2 + w3p3).
- Its two callers are the locomotion routine (the drive's damping, the speed
  along the heading, the vertical run penalty) and the lift servo (its
  damping takes y), both on the character's root with the weights of its own
  origin.
- The post-step copies the previous set into the third and the current into
  the previous; the step does not write the previous set. Between ticks the
  third set is where the last tick started.
- The muscles-off clip sampler's snap writes the current set from the pose,
  then copies it into the previous set and that into the third.

## What Changes

- exa_physics: `Ragdoll::root_moved` (read by the locomotion stage and the lift
  servo) is `Part::two_step_point` of the root's own origin; the lift servo's
  vertical displacement is its y; `snap_part` writes the third set too.

## Impact

The same quantity as before between ticks, summed in the original's order:
the duel and exanima01 traces differ from main by float rounding from the
first ticks (1e-5 relative), which then grows through the bodies' motion and
AI timing.
