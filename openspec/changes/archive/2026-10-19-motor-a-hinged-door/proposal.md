# Proposal

## Why

Phase 2 creaks a hinge and unlatches it under a pull. A toggle or an open command does not yet drive the door toward its aim.

## What Changes

- The aim is `(1 − fraction) · range · sign + closed`. Fraction 1 is shut and 0 is open.
- A plain step is the absolute error over the range, times 0.001, plus 0.0003. A motorised door steps 0.0003, and a fast one 0.005.
- The drive moves that far toward the aim. A toggle arrives within its step. An open command arrives within 0.1 rad.
- A countdown below 2 ends the command after this tick's drive. A door that bars itself latches on arrival and keeps the command.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the hinged door's motor.

## Impact

- `exa_physics::door`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the hinged door step's toggle and open commands. No format field changes, so the corpus step is a skip.
- Out of scope: the sliding door, the partner of a double door, and the slam's random pitch.
