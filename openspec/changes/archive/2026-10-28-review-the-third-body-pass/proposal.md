# Proposal

## Why

The third DeepSeek pass on the body dynamics class (lane `motiledyn3`,
docs/PLAN.md Phase 3, Bodies: skeleton, physics, control) added and
changed body-tick rules: the footwork reach, the stance score's side
axis, the step trigger, an imbalance accumulator, the walking ground
weights, an early gait plant, the eases, the hold cut-off, a limp-body
bracket and a shock hold. Its review checked each rule against the
original's instructions and found several wrong (the walking ground was
off by a factor of 100 and its stored difference grew 100-fold a tick;
the hold cut-off lost a leg bone; the new step target's height and a NaN
balance score took the wrong branch; several constants were single where
the original's are double).

## What Changes

- physical-body: Gait plant phase gains the early plant and its double
  compares; Stance score reads the character frame's side axis and the
  original's arithmetic; new Walking ground weights, Imbalance
  accumulator and Footwork reach.
- motion-playback: new Ease arithmetic and Hold bones.
- Code: corrections listed in the review commits; no new wiring beyond
  the lane's (the walking ground weights, the early plant, the footwork
  reach, the side axis, the kick plant and the step pitch rate run in the
  body tick).

## Evidence

The balance, feet, stance-score, living-sampler, capture and ease
routines' instruction listings in the motiledyn decompilation pack, and
the reach recomputation run on equipment changes.

## Out of scope

The locomotion leg IK, the Ground Query sole floor, the footstep sounds,
the sampler IK chain and stand-height block, the limp sampler's particle
snap, the reach penalty's double sum, and a caller for the queued step,
the shock hold and the limp bracket.
