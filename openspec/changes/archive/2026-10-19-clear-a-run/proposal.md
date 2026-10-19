# Proposal

## Why

Phase 6 slides a body along a path and aims 100 cm ahead in the scene. Near the end it does not yet clear the run bit or soft-stop.

## What Changes

- Urgency of 0 or less sets the run bit until the carrot fraction, read as a double, is past 0.99 on the last segment.
- Past that, the run bit clears, and a 3-D squared distance under 900 from the ground to the carrot soft-stops. Exactly 900 does not.
- Urgency of 1 or more clears the run bit and soft-stops under 100 cm of the last point. Exactly 100 cm does not.
- A soft stop sets the stopping bit, clears free walk and following, and pulls a far carrot back to 8 times the speed plus 20 along the facing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-steering`: the run bit and the soft stop at the end of a path.

## Impact

- `exa_ai::path`, after the carrot and the out-of-scene slide.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the arrival branch of path following and the soft stop it calls. No format field changes, so the corpus step is a skip.
- Out of scope: the line walk and its re-plan flag.
