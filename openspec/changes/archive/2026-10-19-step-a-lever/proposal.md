# Proposal

## Why

Phase 2 opens a door and waits out its unlock. A lever does not yet creak, step toward a detent, or signal when it arrives.

## What Changes

- A lever's creak smooths the arm velocity by half and stops at a squared speed of 0.015. Past that, the volume is 0.2 times the square root of the energy and the pitch is 0.2 times the energy plus 0.7, at most 1.
- The drive moves 0.02 rad toward the target and holds once the gap is within 0.02. Held, the target is the hand. Forced, it is the off detent. Returning, it is the stored detent. Otherwise it is the nearest.
- Arriving more than 0.1 rad from the last detent reports the rest flag at the first detent and its opposite at the second. A report of 0 signals 0 unless inverted, and stops a return. A report of 1 signals 1, or −1 when forced and inverted, and arms a timed return. A linked light fades out on 0 and in on 1.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the lever's step.

## Impact

- `exa_physics::lever`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the lever step. No format field changes, so the corpus step is a skip.
- Out of scope: the return's later tick, the arm nudge, and the shared environment that stops the lever's body.
