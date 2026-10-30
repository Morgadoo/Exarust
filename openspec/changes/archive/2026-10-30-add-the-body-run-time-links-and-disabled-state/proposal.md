# Proposal

## Why

Next pass on the tetrahedral rigid body (docs/PLAN.md Phase 3, physics and
bodies): the run-time link adder, the disabled body state, the two-step
point transform and the tree-wide tick take were missing from the body
kernel.

Evidence: the original's assembly for the link adder and the link
registration it calls, the share routine, the two weighted-point helpers,
the Verlet step and post-step particle rolls, the tick take, the clone and
its unshare routine, and the state-bit tests in the sector's pair and
level passes and in the link iteration (REA and the objdump listing),
with the pack's pseudo-code used for structure only.

## What Changes

- `Ragdoll::add_link`: a body link from a point on each part, weighted
  over the rest particles; the two link-mobility shares, the pivot
  softness 0.001, the body and code flags; the slack zeroed at or under
  the general epsilon (cm) or for a NaN; the measured length overwritten
  with zero. `Ragdoll::add_world_link` is the scene-node branch (code flag
  only). Not wired: no Exarust caller adds links after a body is built.
- `Part::two_step_point`: now less the third particle set, each weighted
  sum paired as the original's helper sums it; the set rolls each
  post-step and on the first tick of sleep. No caller.
- `Ragdoll::set_disabled`: the pair stage and the level detection skip a
  disabled body and a bite on it breaks (wired into those stages; nothing
  sets the flag, as there is no cursor carry).
- `Ragdoll::take_ticks`: subtract and hold at one; used by the frozen step
  and a dormant living body.
- A clone copies its rest particles (the sharing the earlier pass added
  is removed: the original shares only the collision-sphere array, and its
  unshare routine has no caller).

## Out of scope

- The adder's reciprocal mobility sum, the record field no reader uses.
- The cursor carry that would set the disabled state, its zero-gravity
  environment and its asleep bits.
- The per-root iteration mark and the step's unread counter.
