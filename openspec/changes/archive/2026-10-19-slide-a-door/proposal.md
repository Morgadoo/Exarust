# Proposal

## Why

Phase 2 drives a hinged door toward its aim. A sliding door does not yet travel, creak, or unlatch under a pull.

## What Changes

- A sliding creak stops at stored energy 0.015. Past that, the volume is 0.2 times the square root of the energy and the pitch is 0.2 times the energy plus 0.7, at most 1.
- A motorised open drops the fraction by 0.01 a tick, with smoothing 0.1, and stops below 0.01. A close adds 0.075, with smoothing 0.05, and latches once the fraction passes 0.99.
- Shut and not held, the fraction stays 1 with smoothing 0.5. Held below 0.995, a pull of 4 unlatches on the seventh tick from 20.
- A plain open sets the fraction to 0.6 for 80 ticks. A frozen door keeps its stored fraction and stays passable. A hand point past 60 cm is pulled back to 60.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the sliding door's travel.

## Impact

- `exa_physics::door`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the sliding door step. No format field changes, so the corpus step is a skip.
- Out of scope: the partner of a double door, the lock and its key, the slam's random pitch, and the link that pulls the body.
