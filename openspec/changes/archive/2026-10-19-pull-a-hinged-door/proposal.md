# Proposal

## Why

Phase 2 hides a room behind a shut door. The door itself does not creak, does not open under a pull, and does not take the shove that starts it swinging.

## What Changes

- The hinge creak smooths the handle velocity by half and stops at a squared speed of 0.015. Past that, the volume is 0.075 times the square root of the speed and the pitch is 0.022 times the speed plus 0.7, at most 1.
- A shut door's pull count rises by 1 up to 20, then falls by 4 while it is held, not latched, and still short of closed. Below 0 it unlatches. From 20, the seventh pull does that.
- Opening shoves the handle 3 cm along the door's facing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the hinged door's creak, pull, and open shove.

## Impact

- `exa_physics::door`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the hinged door step and the open shove. No format field changes, so the corpus step is a skip.
- Out of scope: the motor, the sliding door, the partner of a double door, and the slam's random pitch.
